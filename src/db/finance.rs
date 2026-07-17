use rayon::prelude::*;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Result, Transaction, params};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, io, path::Path, sync::Arc};
use time::Date;

use crate::db::parse::ParseTbf;

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
        if data.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "TBF 数据中没有完整记录"));
        }

        data.into_par_iter().map(|m| serde_json::from_str(&m).map_err(io::Error::other)).collect()
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

    pub fn open_read_only<P: AsRef<Path>>(database_path: P) -> Result<Self> {
        let database = Connection::open_with_flags(database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

        Ok(Self { database })
    }

    pub fn create_database(&self) -> Result<()> {
        self.database.execute(include_str!("sql/finance_create.sql"), [])?;

        Ok(())
    }

    pub fn table_exists(&self, table_name: &str) -> Result<bool> {
        self.database
            .query_row(include_str!("sql/table_exists.sql"), params![table_name], |row| row.get(0))
    }

    pub fn clear_data(&self) -> Result<()> {
        self.database.execute("DELETE FROM financial", [])?;

        Ok(())
    }

    pub fn query(&self, start: Date, end: Date) -> Result<Vec<Finance>> {
        let start = start.to_string();
        let end = end.saturating_add(time::Duration::days(1)).to_string();

        let mut stmt = self.database.prepare(include_str!("sql/finance_query_range.sql"))?;

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

    /// 查询全部财务数据，结果按时间升序排列。
    pub fn query_all(&self) -> Result<Vec<Finance>> {
        let mut stmt = self.database.prepare(include_str!("sql/finance_query_all.sql"))?;
        let rows = stmt.query_map([], |row| {
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
    /// 查询时间最新的一条财务数据。
    pub fn query_latest(&self) -> Result<Option<Finance>> {
        self.database
            .query_row(include_str!("sql/finance_query_latest.sql"), [], |row| {
                Ok(Finance {
                    datetime: Arc::from(row.get::<_, String>(0)?),
                    total_shares: row.get(1)?,
                    float_shares: row.get(2)?,
                    total_market: row.get(3)?,
                    float_market: row.get(4)?,
                })
            })
            .optional()
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

    pub fn add_batch(&mut self, data: &[Finance]) -> Result<()> {
        let transaction = self.database.transaction()?;
        add_finance_batch(&transaction, data)?;
        transaction.commit()
    }

    pub fn replace_all(&mut self, data: &[Finance]) -> Result<()> {
        let transaction = self.database.transaction()?;
        transaction.execute(include_str!("sql/finance_create.sql"), [])?;
        transaction.execute("DELETE FROM financial", [])?;
        add_finance_batch(&transaction, data)?;
        transaction.commit()
    }
}

fn add_finance_batch(transaction: &Transaction<'_>, data: &[Finance]) -> Result<()> {
    let mut statement = transaction.prepare_cached(include_str!("sql/finance_insert.sql"))?;
    for financial in data {
        statement.execute(params![
            financial.datetime.as_ref(),
            financial.total_shares,
            financial.float_shares,
            financial.total_market,
            financial.float_market,
        ])?;
    }

    Ok(())
}

/// 解析tbf财务数据并保存（每个股票一个独立数据库）
pub fn tbf_to_finance(input: &str, output: &str) -> io::Result<()> {
    fs::create_dir_all(output).map_err(|e| io::Error::other(format!("创建财务输出目录失败 {output}: {e}")))?;

    let results: Vec<_> = fs::read_dir(input)
        .map_err(|e| io::Error::other(format!("读取财务输入目录失败 {input}: {e}")))?
        .par_bridge()
        .map(|entry| -> io::Result<_> {
            let entry = entry.map_err(|e| io::Error::other(format!("读取财务目录项失败 {input}: {e}")))?;
            let path = entry.path();
            let code = path
                .file_stem()
                .ok_or_else(|| io::Error::other(format!("财务文件名缺少 stem: {}", path.display())))?
                .to_string_lossy()
                .to_string();
            let display = path.display().to_string();

            let mut pt = ParseTbf::new("<begin>", "</end>");
            let data = pt
                .parse(&path)
                .map_err(|e| io::Error::other(format!("TBF财务边界解析失败 {display}: {e}")))?;
            let finance = Finance::parse(data).map_err(|e| io::Error::other(format!("Finance JSON解析失败 {display}: {e}")))?;
            Ok((code, finance))
        })
        .collect::<io::Result<Vec<_>>>()?;

    for (code, finance) in &results {
        let db_path = Path::new(output).join(format!("{code}.db"));
        let mut db = FinanceDB::new(&db_path).map_err(|e| io::Error::other(format!("打开财务数据库失败 {}: {e}", db_path.display())))?;
        db.replace_all(finance)
            .map_err(|e| io::Error::other(format!("刷新财务数据失败 {}: {e}", db_path.display())))?;
    }

    Ok(())
}
