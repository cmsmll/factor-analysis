use rayon::prelude::*;
use rusqlite::{Connection, OptionalExtension, Result, params, types::Type};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, io, path::Path, sync::Arc, time::Instant};

use crate::db::parse::ParseTbf;

/// 股票元数据
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub exchange: String,     // 交易所
    pub name: Arc<str>,       // 名称
    pub code: Arc<str>,       // 代码
    pub prov: String,         // 省份
    pub city: String,         // 城市
    pub SW1: String,          // 申万一级
    pub SW2: String,          // 申万二级
    pub SW3: String,          // 申万三级
    pub indice: Vec<String>,  // 入选指数
    pub listing_date: String, // 上市时间
}

impl Metadata {
    pub fn parse_first(data: BTreeSet<String>) -> io::Result<Self> {
        let data = data.into_iter().next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "metadata tbf data is empty")
        })?;

        serde_json::from_str(&data).map_err(io::Error::other)
    }
}

pub struct MetadataDb {
    database: Connection,
}

impl MetadataDb {
    pub fn new<P: AsRef<Path>>(database_path: P) -> Result<Self> {
        let database = Connection::open(database_path)?;

        Ok(Self { database })
    }

    pub fn create_database(&self) -> Result<()> {
        self.database
            .execute(include_str!("sql/metadata_create.sql"), [])?;

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
        self.database.execute("DELETE FROM metadata", [])?;

        Ok(())
    }

    pub fn add_data(&self, metadata: &Metadata) -> Result<()> {
        let indice = serde_json::to_string(&metadata.indice).map_err(json_to_sql_error)?;

        self.database.execute(
            include_str!("sql/metadata_insert.sql"),
            params![
                metadata.code.as_ref(),
                metadata.exchange,
                metadata.name.as_ref(),
                metadata.prov,
                metadata.city,
                metadata.SW1,
                metadata.SW2,
                metadata.SW3,
                indice,
                metadata.listing_date,
            ],
        )?;

        Ok(())
    }

    pub fn add_batch(&mut self, data: &[Metadata]) -> Result<()> {
        let transaction = self.database.transaction()?;
        for metadata in data {
            add_metadata(&transaction, metadata)?;
        }
        transaction.commit()?;

        Ok(())
    }

    pub fn query(&self, code: &str) -> Result<Option<Metadata>> {
        self.database
            .query_row(
                include_str!("sql/metadata_query_by_code.sql"),
                params![code],
                metadata_from_row,
            )
            .optional()
    }

    pub fn query_all(&self) -> Result<Vec<Metadata>> {
        let mut stmt = self
            .database
            .prepare(include_str!("sql/metadata_query_all.sql"))?;

        let rows = stmt.query_map([], metadata_from_row)?;

        rows.collect()
    }
}

fn add_metadata(database: &Connection, metadata: &Metadata) -> Result<()> {
    let indice = serde_json::to_string(&metadata.indice).map_err(json_to_sql_error)?;

    database.execute(
        include_str!("sql/metadata_insert.sql"),
        params![
            metadata.code.as_ref(),
            metadata.exchange,
            metadata.name.as_ref(),
            metadata.prov,
            metadata.city,
            metadata.SW1,
            metadata.SW2,
            metadata.SW3,
            indice,
            metadata.listing_date,
        ],
    )?;

    Ok(())
}

fn metadata_from_row(row: &rusqlite::Row<'_>) -> Result<Metadata> {
    let indice: String = row.get(8)?;

    Ok(Metadata {
        exchange: row.get(0)?,
        name: Arc::from(row.get::<_, String>(1)?),
        code: Arc::from(row.get::<_, String>(2)?),
        prov: row.get(3)?,
        city: row.get(4)?,
        SW1: row.get(5)?,
        SW2: row.get(6)?,
        SW3: row.get(7)?,
        indice: serde_json::from_str(&indice).map_err(json_from_sql_error)?,
        listing_date: row.get(9)?,
    })
}

fn json_to_sql_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

fn json_from_sql_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(8, Type::Text, Box::new(error))
}

/// 解析tbf元数据并保存到一个数据库
pub fn tbf_to_metadata(input: &str, output: &str) -> io::Result<()> {
    let total_start = Instant::now();
    if let Some(parent) = Path::new(output).parent() {
        fs::create_dir_all(parent).map_err(|e| {
            io::Error::other(format!("创建元数据输出目录失败 {}: {e}", parent.display()))
        })?;
    }

    let parse_start = Instant::now();
    let metadata: Vec<_> = fs::read_dir(input)
        .map_err(|e| io::Error::other(format!("读取元数据输入目录失败 {input}: {e}")))?
        .par_bridge()
        .map(|entry| -> io::Result<_> {
            let entry = entry
                .map_err(|e| io::Error::other(format!("读取元数据目录项失败 {input}: {e}")))?;
            let path = entry.path();
            let display = path.display().to_string();

            let mut pt = ParseTbf::new("<begin>", "</end>");
            let data = pt
                .parse(&path)
                .map_err(|e| io::Error::other(format!("TBF元数据边界解析失败 {display}: {e}")))?;
            Metadata::parse_first(data)
                .map_err(|e| io::Error::other(format!("Metadata JSON解析失败 {display}: {e}")))
        })
        .collect::<io::Result<Vec<_>>>()?;
    println!(
        "tbf_to_metadata 解析完成: input={input}, 文件数={}, 耗时={:?}",
        metadata.len(),
        parse_start.elapsed()
    );

    let write_start = Instant::now();
    let mut db = MetadataDb::new(output)
        .map_err(|e| io::Error::other(format!("打开元数据数据库失败 {output}: {e}")))?;
    if db
        .table_exists("metadata")
        .map_err(|e| io::Error::other(format!("判断元数据表是否存在失败 {output}: {e}")))?
    {
        db.clear_data()
            .map_err(|e| io::Error::other(format!("清空元数据表失败 {output}: {e}")))?;
    } else {
        db.create_database()
            .map_err(|e| io::Error::other(format!("创建元数据表失败 {output}: {e}")))?;
    }
    db.add_batch(&metadata)
        .map_err(|e| io::Error::other(format!("写入元数据失败 {output}: {e}")))?;
    println!(
        "tbf_to_metadata 写入完成: output={output}, 记录数={}, 耗时={:?}, 总耗时={:?}",
        metadata.len(),
        write_start.elapsed(),
        total_start.elapsed()
    );

    Ok(())
}
