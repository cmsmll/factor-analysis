use rayon::prelude::*;
use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    fmt::Display,
    fs, io,
    path::Path,
    sync::Arc,
    time::Instant,
};
use time::Date;

use crate::db::parse::ParseTbf;

const MARKET_QUERY_RANGE_SQL: &str = r#"
SELECT
    datetime,
    change_percent,
    open,
    close,
    high,
    low,
    volume,
    turnover,
    turnover_rate,
    is_st
FROM market_data
WHERE datetime >= ?1 AND datetime < ?2
ORDER BY datetime;
"#;

/// 行情数据
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

        let mut stmt = self.database.prepare(MARKET_QUERY_RANGE_SQL)?;

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

    pub fn query_with_set(&self, start: Date, end: Date, set: &mut BTreeSet<Arc<str>>) -> Result<(Vec<Arc<MarketData>>, HashMap<Arc<str>, usize>)> {
        let start = start.to_string();
        let end = end.saturating_add(time::Duration::days(1)).to_string();

        let mut stmt = self.database.prepare(MARKET_QUERY_RANGE_SQL)?;

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

    pub fn query_with_table(
        &self,
        start: Date,
        end: Date,
        table: &mut BTreeSet<Arc<str>>,
    ) -> Result<(Vec<Arc<MarketData>>, HashMap<Arc<str>, usize>)> {
        self.query_with_set(start, end, table)
    }

    pub fn add_batch(&self, data: &[MarketData]) -> Result<()> {
        self.database.execute("BEGIN TRANSACTION", [])?;
        for md in data {
            self.add_data(md)?;
        }
        self.database.execute("COMMIT", [])?;

        Ok(())
    }
}

/// 解析tbf数据并保存（每个股票一个独立数据库）
pub fn tbf_to_market(input: &str, output: &str) -> io::Result<()> {
    let total_start = Instant::now();
    fs::create_dir_all(output).map_err(|e| io::Error::other(format!("创建行情输出目录失败 {output}: {e}")))?;

    // 先并行解析所有文件数据，收集到 Vec 中
    let parse_start = Instant::now();
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
    println!(
        "tbf_to_market 解析完成: input={input}, 文件数={}, 耗时={:?}",
        results.len(),
        parse_start.elapsed()
    );

    // 每个股票独立写入各自的数据库
    let write_start = Instant::now();
    for (code, md) in &results {
        let db_path = Path::new(output).join(format!("{code}.db"));
        let db = MarketDataDb::new(&db_path).map_err(|e| io::Error::other(format!("打开行情数据库失败 {}: {e}", db_path.display())))?;
        if db
            .table_exists("market_data")
            .map_err(|e| io::Error::other(format!("判断行情表是否存在失败 {}: {e}", db_path.display())))?
        {
            db.clear_data()
                .map_err(|e| io::Error::other(format!("清空行情表失败 {}: {e}", db_path.display())))?;
        } else {
            db.create_database()
                .map_err(|e| io::Error::other(format!("创建行情表失败 {}: {e}", db_path.display())))?;
        }
        db.add_batch(md)
            .map_err(|e| io::Error::other(format!("写入行情数据失败 {}: {e}", db_path.display())))?;
    }
    println!(
        "tbf_to_market 写入完成: output={output}, 数据库数={}, 耗时={:?}, 总耗时={:?}",
        results.len(),
        write_start.elapsed(),
        total_start.elapsed()
    );

    Ok(())
}
