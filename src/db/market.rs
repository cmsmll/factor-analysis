use rayon::prelude::*;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Result, Transaction, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    fmt::Display,
    fs, io,
    path::Path,
    sync::Arc,
};
use time::Date;

use crate::db::parse::ParseTbf;

/// 行情数据
pub type MarketDataList = Vec<Arc<MarketData>>;
pub type MarketIndexTable = HashMap<Arc<str>, usize>;
pub type MarketQueryResult = (MarketDataList, MarketIndexTable);

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MarketData {
    /// 日期时间（例如：2025-03-15 14:30:00）
    pub datetime: Arc<str>,
    /// 涨幅（百分比）
    pub change_percent: f64,
    /// 开盘价
    pub open: f64,
    /// 收盘价
    pub close: f64,
    /// 最高价
    pub high: f64,
    /// 最低价
    pub low: f64,
    /// 成交量
    pub volume: f64,
    /// 成交额
    pub turnover: f64,
    /// 换手率（百分比）
    pub turnover_rate: f64,
    /// 是否为ST
    pub is_st: bool,
}

impl MarketData {
    pub fn parse(data: BTreeSet<String>) -> io::Result<Vec<Self>> {
        if data.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "TBF 数据中没有完整记录"));
        }

        data.into_par_iter().map(|m| serde_json::from_str(&m).map_err(io::Error::other)).collect()
    }

    pub fn table_header() {
        println!("┌─────────────────────┬──────────┬───────────┬───────────┬───────────┬───────────┬───────────┬───────────┬───────────┬──────┐");
        println!(
            "│ {:^18}│ {:>5}  │   开盘价  │   收盘价  │   最高价  │   最低价  │   成交量  │   成交额  │   换手率  │  ST  │",
            "时间", "涨幅"
        );
        Self::table_middle();
    }

    pub fn table_middle() {
        println!("├─────────────────────┼──────────┼───────────┼───────────┼───────────┼───────────┼───────────┼───────────┼───────────┼──────┤");
    }

    pub fn table_footer() {
        println!("└─────────────────────┴──────────┴───────────┴───────────┴───────────┴───────────┴───────────┴───────────┴───────────┴──────┘");
    }

    pub fn table_display(data: &[MarketData]) {
        if data.is_empty() {
            return;
        }

        let mut prev = data[0].datetime.clone();
        MarketData::table_header();
        for item in data {
            if item.datetime[0..10] != prev[0..10] {
                MarketData::table_middle();
            }
            prev = item.datetime.clone();
            println!("{item}")
        }
        MarketData::table_footer();
    }
}

pub const YI: f64 = 1e8; // 1 × 10⁸
pub const WAN: f64 = 1e4; // 1 × 10⁴

impl Display for MarketData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "│ {:20}", self.datetime)?; // 时间
        write!(
            f,
            "│{} {:>7.2}% \x1b[0m",
            color_for_number(self.change_percent),
            self.change_percent * 100.0
        )?; // 涨幅
        write!(f, "│ {:>9.2} ", self.open)?; // 开盘价
        write!(f, "│ {:>9.2} ", self.close)?; // 收盘价
        write!(f, "│ {:>9.2} ", self.high)?; // 最高价
        write!(f, "│ {:>9.2} ", self.low)?; // 最低价
        write!(f, "│ {:>7.0}万 ", self.volume / WAN)?; // 成交量
        write!(f, "│ {:>7.2}亿 ", self.turnover / YI)?; // 成交额
        write!(f, "│ {:>7.2}%  ", self.turnover_rate)?; // 换手率 
        write!(f, "│  {}  │", if self.is_st { "是" } else { "否" }) // 是否为ST 
    }
}

fn color_for_number(n: f64) -> &'static str {
    if n == 0.0 {
        "\x1b[39m" // white
    } else if n < 0.0 {
        "\x1b[32m" // green
    } else {
        "\x1b[31m" // red
    }
}

/// 行情数据库
pub struct MarketDataDb {
    database: Connection,
}

impl MarketDataDb {
    pub fn new<P: AsRef<Path>>(database_path: P) -> Result<Self> {
        let database = Connection::open(database_path)?;

        Ok(Self { database })
    }

    pub fn open_read_only<P: AsRef<Path>>(database_path: P) -> Result<Self> {
        let database = Connection::open_with_flags(database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

        Ok(Self { database })
    }

    pub fn create_database(&self) -> Result<()> {
        self.database.execute(include_str!("sql/market_create.sql"), [])?;

        Ok(())
    }

    pub fn table_exists(&self, table_name: &str) -> Result<bool> {
        self.database
            .query_row(include_str!("sql/table_exists.sql"), params![table_name], |row| row.get(0))
    }

    pub fn clear_data(&self) -> Result<()> {
        self.database.execute("DELETE FROM market_data", [])?;

        Ok(())
    }

    pub fn add_data(&self, md: &MarketData) -> Result<()> {
        self.database.execute(
            include_str!("sql/market_insert.sql"),
            params![
                md.datetime.as_ref(),
                md.change_percent,
                md.open,
                md.close,
                md.high,
                md.low,
                md.volume,
                md.turnover,
                md.turnover_rate,
                md.is_st
            ],
        )?;

        Ok(())
    }

    pub fn query(&self, start: Date, end: Date) -> Result<Vec<MarketData>> {
        let start = start.to_string();
        let end = end.saturating_add(time::Duration::days(1)).to_string();

        let mut stmt = self.database.prepare(include_str!("sql/market_query_range.sql"))?;

        let rows = stmt.query_map(params![start, end], |row| {
            Ok(MarketData {
                datetime: Arc::from(row.get::<_, String>(0)?),
                change_percent: row.get(1)?,
                open: row.get(2)?,
                close: row.get(3)?,
                high: row.get(4)?,
                low: row.get(5)?,
                volume: row.get(6)?,
                turnover: row.get(7)?,
                turnover_rate: row.get(8)?,
                is_st: row.get(9)?,
            })
        })?;

        rows.collect()
    }

    /// 查询全部行情数据，结果按时间升序排列。
    pub fn query_all(&self) -> Result<Vec<MarketData>> {
        let mut stmt = self.database.prepare(include_str!("sql/market_query_all.sql"))?;
        let rows = stmt.query_map([], |row| {
            Ok(MarketData {
                datetime: Arc::from(row.get::<_, String>(0)?),
                change_percent: row.get(1)?,
                open: row.get(2)?,
                close: row.get(3)?,
                high: row.get(4)?,
                low: row.get(5)?,
                volume: row.get(6)?,
                turnover: row.get(7)?,
                turnover_rate: row.get(8)?,
                is_st: row.get(9)?,
            })
        })?;

        rows.collect()
    }
    /// 查询时间最新的一条行情数据。
    pub fn query_latest(&self) -> Result<Option<MarketData>> {
        self.database
            .query_row(include_str!("sql/market_query_latest.sql"), [], |row| {
                Ok(MarketData {
                    datetime: Arc::from(row.get::<_, String>(0)?),
                    change_percent: row.get(1)?,
                    open: row.get(2)?,
                    close: row.get(3)?,
                    high: row.get(4)?,
                    low: row.get(5)?,
                    volume: row.get(6)?,
                    turnover: row.get(7)?,
                    turnover_rate: row.get(8)?,
                    is_st: row.get(9)?,
                })
            })
            .optional()
    }

    pub fn query_with_set(&self, start: Date, end: Date, set: &mut BTreeSet<Arc<str>>) -> Result<MarketQueryResult> {
        let start = start.to_string();
        let end = end.saturating_add(time::Duration::days(1)).to_string();

        let mut stmt = self.database.prepare(include_str!("sql/market_query_range.sql"))?;

        // 时间索引表
        let mut table = HashMap::default();
        let rows = stmt.query_map(params![start, end], |row| {
            let mut datetime: Arc<str> = Arc::from(row.get::<_, String>(0)?);
            if let Some(arc_dt) = set.get(datetime.as_ref()) {
                datetime = arc_dt.clone();
            } else {
                set.insert(datetime.clone());
            };
            table.insert(datetime.clone(), table.len());

            Ok(Arc::new(MarketData {
                datetime,
                change_percent: row.get(1)?,
                open: row.get(2)?,
                close: row.get(3)?,
                high: row.get(4)?,
                low: row.get(5)?,
                volume: row.get(6)?,
                turnover: row.get(7)?,
                turnover_rate: row.get(8)?,
                is_st: row.get(9)?,
            }))
        })?;

        Ok((rows.collect::<Result<Vec<Arc<MarketData>>, rusqlite::Error>>()?, table))
    }

    pub fn query_with_table(&self, start: Date, end: Date, table: &mut BTreeSet<Arc<str>>) -> Result<MarketQueryResult> {
        self.query_with_set(start, end, table)
    }

    pub fn add_batch(&mut self, data: &[MarketData]) -> Result<()> {
        let transaction = self.database.transaction()?;
        add_market_batch(&transaction, data)?;
        transaction.commit()
    }

    pub fn replace_all(&mut self, data: &[MarketData]) -> Result<()> {
        let transaction = self.database.transaction()?;
        transaction.execute(include_str!("sql/market_create.sql"), [])?;
        transaction.execute("DELETE FROM market_data", [])?;
        add_market_batch(&transaction, data)?;
        transaction.commit()
    }
}

fn add_market_batch(transaction: &Transaction<'_>, data: &[MarketData]) -> Result<()> {
    let mut statement = transaction.prepare_cached(include_str!("sql/market_insert.sql"))?;
    for md in data {
        statement.execute(params![
            md.datetime.as_ref(),
            md.change_percent,
            md.open,
            md.close,
            md.high,
            md.low,
            md.volume,
            md.turnover,
            md.turnover_rate,
            md.is_st,
        ])?;
    }

    Ok(())
}

/// 解析tbf数据并保存（每个股票一个独立数据库）
pub fn tbf_to_market(input: &str, output: &str) -> io::Result<()> {
    fs::create_dir_all(output).map_err(|e| io::Error::other(format!("创建行情输出目录失败 {output}: {e}")))?;

    // 先并行解析所有文件数据，收集到 Vec 中
    let results: Vec<_> = fs::read_dir(input)
        .map_err(|e| io::Error::other(format!("读取行情输入目录失败 {input}: {e}")))?
        .par_bridge()
        .map(|entry| -> io::Result<_> {
            let entry = entry.map_err(|e| io::Error::other(format!("读取行情目录项失败 {input}: {e}")))?;
            let path = entry.path();
            let code = path
                .file_stem()
                .ok_or_else(|| io::Error::other(format!("行情文件名缺少 stem: {}", path.display())))?
                .to_string_lossy()
                .to_string();
            let display = path.display().to_string();

            let mut pt = ParseTbf::new("<begin>", "</end>");
            let data = pt
                .parse(&path)
                .map_err(|e| io::Error::other(format!("TBF行情边界解析失败 {display}: {e}")))?;
            let md = MarketData::parse(data).map_err(|e| io::Error::other(format!("MarketData JSON解析失败 {display}: {e}")))?;
            Ok((code, md))
        })
        .collect::<io::Result<Vec<_>>>()?;

    // 每个股票独立写入各自的数据库
    for (code, md) in &results {
        let db_path = Path::new(output).join(format!("{code}.db"));
        let mut db = MarketDataDb::new(&db_path).map_err(|e| io::Error::other(format!("打开行情数据库失败 {}: {e}", db_path.display())))?;
        db.replace_all(md)
            .map_err(|e| io::Error::other(format!("刷新行情数据失败 {}: {e}", db_path.display())))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tempfile::tempdir;
    use time::{Date, Month};

    use super::*;

    fn date(day: u8) -> Date {
        Date::from_calendar_date(2025, Month::January, day).unwrap()
    }

    fn market(datetime: &str, close: f64) -> MarketData {
        MarketData {
            datetime: Arc::from(datetime),
            change_percent: 0.01,
            open: close - 1.0,
            close,
            high: close + 1.0,
            low: close - 2.0,
            volume: 100.0,
            turnover: 1_000.0,
            turnover_rate: 0.02,
            is_st: false,
        }
    }

    // 测试范围查询包含 end 当天，并排除 end 的下一天。
    #[test]
    fn query_range_includes_end_date() {
        let directory = tempdir().unwrap();
        let database_path = directory.path().join("market.db");
        let mut db = MarketDataDb::new(database_path).unwrap();
        db.replace_all(&[market("2025-01-01", 10.0), market("2025-01-02", 11.0), market("2025-01-03", 12.0)])
            .unwrap();

        let data = db.query(date(1), date(2)).unwrap();

        assert_eq!(data.len(), 2);
        assert_eq!(data[0].datetime.as_ref(), "2025-01-01");
        assert_eq!(data[1].datetime.as_ref(), "2025-01-02");
    }
}
