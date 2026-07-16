use rayon::prelude::*;
use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, io, path::Path, sync::Arc, time::Instant};
use time::Date;

use crate::db::parse::ParseTbf;

const FINANCE_QUERY_RANGE_SQL: &str = r#"
SELECT
    datetime,
    total_shares,
    float_shares,
    total_market,
    float_market
FROM financial
WHERE datetime >= ?1 AND datetime < ?2
ORDER BY datetime;
"#;

/// 财务数据
/// 财务数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Finance {
    pub datetime: Arc<str>,
    /// 总股本（单位：股）
    pub total_shares: f64,
    /// 流通股本（单位：股）
    pub float_shares: f64,
    /// 总市值（单位：元）
    pub total_market: f64,
    /// 流通市值（单位：元）
    pub float_market: f64,
}

impl Finance {
    pub fn parse(data: BTreeSet<String>) -> io::Result<Vec<Self>> {
        data.into_par_iter()
            .map(|m| serde_json::from_str(&m).map_err(io::Error::other))
            .collect()
    }

    pub fn same_data(&self, other: &Self) -> bool {
        self.datetime == other.datetime
            && self.total_shares.to_bits() == other.total_shares.to_bits()
            && self.float_shares.to_bits() == other.float_shares.to_bits()
            && self.total_market.to_bits() == other.total_market.to_bits()
            && self.float_market.to_bits() == other.float_market.to_bits()
    }
}

/// 财务数据库
pub struct FinanceDB {
    database: Connection,
}

impl FinanceDB {
    pub fn new<P: AsRef<Path>>(database_path: P) -> Result<Self> {
        let database = Connection::open(database_path)?;

        Ok(Self { database })
    }

    pub fn create_database(&self) -> Result<()> {
        self.database
            .execute(include_str!("sql/finance_create.sql"), [])?;

        Ok(())
    }

    pub fn table_exists(&self, table_name: &str) -> Result<bool> {
        self.database.query_row(
            include_str!("sql/table_exists.sql"),
            params![table_name],
            |row| row.get(0),
        )
    }

    pub fn clear_data(&self) -> Result<()> {
        self.database.execute("DELETE FROM financial", [])?;

        Ok(())
    }

    pub fn query(&self, start: Date, end: Date) -> Result<Vec<Finance>> {
        let start = start.to_string();
        let end = end.saturating_add(time::Duration::days(1)).to_string();

        let mut stmt = self.database.prepare(FINANCE_QUERY_RANGE_SQL)?;

        let rows = stmt.query_map(params![start, end], |row| {
            Ok(Finance {
                datetime: Arc::from(row.get::<_, String>(0)?),
                total_shares: row.get(1)?,
                float_shares: row.get(2)?,
                total_market: row.get(3)?,
                float_market: row.get(4)?,
            })
        })?;

        rows.collect()
    }

    pub fn add_data(&self, financial: &Finance) -> Result<()> {
        self.database.execute(
            include_str!("sql/finance_insert.sql"),
            params![
                financial.datetime.as_ref(),
                financial.total_shares,
                financial.float_shares,
                financial.total_market,
                financial.float_market
            ],
        )?;

        Ok(())
    }

    pub fn add_batch(&self, data: &[Finance]) -> Result<()> {
        self.database.execute("BEGIN TRANSACTION", [])?;
        for financial in data {
            self.add_data(financial)?;
        }
        self.database.execute("COMMIT", [])?;

        Ok(())
    }
}

/// 解析tbf财务数据并保存（每个股票一个独立数据库）
pub fn tbf_to_finance(input: &str, output: &str) -> io::Result<()> {
    let total_start = Instant::now();
    fs::create_dir_all(output)
        .map_err(|e| io::Error::other(format!("创建财务输出目录失败 {output}: {e}")))?;

    let parse_start = Instant::now();
    let results: Vec<_> = fs::read_dir(input)
        .map_err(|e| io::Error::other(format!("读取财务输入目录失败 {input}: {e}")))?
        .par_bridge()
        .map(|entry| -> io::Result<_> {
            let entry =
                entry.map_err(|e| io::Error::other(format!("读取财务目录项失败 {input}: {e}")))?;
            let path = entry.path();
            let code = path
                .file_stem()
                .ok_or_else(|| {
                    io::Error::other(format!("财务文件名缺少 stem: {}", path.display()))
                })?
                .to_string_lossy()
                .to_string();
            let display = path.display().to_string();

            let mut pt = ParseTbf::new("<begin>", "</end>");
            let data = pt
                .parse(&path)
                .map_err(|e| io::Error::other(format!("TBF财务边界解析失败 {display}: {e}")))?;
            let finance = Finance::parse(data)
                .map_err(|e| io::Error::other(format!("Finance JSON解析失败 {display}: {e}")))?;
            Ok((code, finance))
        })
        .collect::<io::Result<Vec<_>>>()?;
    println!(
        "tbf_to_finance 解析完成: input={input}, 文件数={}, 耗时={:?}",
        results.len(),
        parse_start.elapsed()
    );

    let write_start = Instant::now();
    for (code, finance) in &results {
        let db_path = Path::new(output).join(format!("{code}.db"));
        let db = FinanceDB::new(&db_path).map_err(|e| {
            io::Error::other(format!("打开财务数据库失败 {}: {e}", db_path.display()))
        })?;
        if db.table_exists("financial").map_err(|e| {
            io::Error::other(format!("判断财务表是否存在失败 {}: {e}", db_path.display()))
        })? {
            db.clear_data().map_err(|e| {
                io::Error::other(format!("清空财务表失败 {}: {e}", db_path.display()))
            })?;
        } else {
            db.create_database().map_err(|e| {
                io::Error::other(format!("创建财务表失败 {}: {e}", db_path.display()))
            })?;
        }
        db.add_batch(finance).map_err(|e| {
            io::Error::other(format!("写入财务数据失败 {}: {e}", db_path.display()))
        })?;
    }
    println!(
        "tbf_to_finance 写入完成: output={output}, 数据库数={}, 耗时={:?}, 总耗时={:?}",
        results.len(),
        write_start.elapsed(),
        total_start.elapsed()
    );

    Ok(())
}
