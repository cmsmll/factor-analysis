use rayon::prelude::*;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Result, Transaction, params, types::Type};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashSet},
    fs, io,
    path::Path,
    sync::Arc,
};

use crate::db::parse::ParseTbf;

/// 股票元数据
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub exchange: String,        // 交易所
    pub name: Arc<str>,          // 名称
    pub code: Arc<str>,          // 代码
    pub prov: String,            // 省份
    pub city: String,            // 城市
    pub SW1: String,             // 申万一级
    pub SW2: String,             // 申万二级
    pub SW3: String,             // 申万三级
    pub indice: HashSet<String>, // 入选指数
    pub listing_date: String,    // 上市时间
}

impl Metadata {
    pub fn parse_first(data: BTreeSet<String>) -> io::Result<Self> {
        let data = data
            .into_iter()
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "metadata tbf data is empty"))?;

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

    pub fn open_read_only<P: AsRef<Path>>(database_path: P) -> Result<Self> {
        let database = Connection::open_with_flags(database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

        Ok(Self { database })
    }

    pub fn create_database(&self) -> Result<()> {
        self.database.execute(include_str!("sql/metadata_create.sql"), [])?;

        Ok(())
    }

    pub fn table_exists(&self, table_name: &str) -> Result<bool> {
        self.database
            .query_row(include_str!("sql/table_exists.sql"), params![table_name], |row| row.get(0))
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
        add_metadata_batch(&transaction, data)?;
        transaction.commit()
    }

    pub fn replace_all(&mut self, data: &[Metadata]) -> Result<()> {
        let transaction = self.database.transaction()?;
        transaction.execute(include_str!("sql/metadata_create.sql"), [])?;
        transaction.execute("DELETE FROM metadata", [])?;
        add_metadata_batch(&transaction, data)?;
        transaction.commit()
    }

    pub fn query(&self, code: &str) -> Result<Option<Metadata>> {
        self.database
            .query_row(include_str!("sql/metadata_query_by_code.sql"), params![code], metadata_from_row)
            .optional()
    }

    pub fn query_all(&self) -> Result<Vec<Metadata>> {
        let mut stmt = self.database.prepare(include_str!("sql/metadata_query_all.sql"))?;

        let rows = stmt.query_map([], metadata_from_row)?;

        rows.collect()
    }
}

fn add_metadata_batch(transaction: &Transaction<'_>, data: &[Metadata]) -> Result<()> {
    let mut statement = transaction.prepare_cached(include_str!("sql/metadata_insert.sql"))?;
    for metadata in data {
        let indice = serde_json::to_string(&metadata.indice).map_err(json_to_sql_error)?;
        statement.execute(params![
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
        ])?;
    }

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
    if let Some(parent) = Path::new(output).parent() {
        fs::create_dir_all(parent).map_err(|e| io::Error::other(format!("创建元数据输出目录失败 {}: {e}", parent.display())))?;
    }

    let metadata: Vec<_> = fs::read_dir(input)
        .map_err(|e| io::Error::other(format!("读取元数据输入目录失败 {input}: {e}")))?
        .par_bridge()
        .map(|entry| -> io::Result<_> {
            let entry = entry.map_err(|e| io::Error::other(format!("读取元数据目录项失败 {input}: {e}")))?;
            let path = entry.path();
            let display = path.display().to_string();

            let mut pt = ParseTbf::new("<begin>", "</end>");
            let data = pt
                .parse(&path)
                .map_err(|e| io::Error::other(format!("TBF元数据边界解析失败 {display}: {e}")))?;
            Metadata::parse_first(data).map_err(|e| io::Error::other(format!("Metadata JSON解析失败 {display}: {e}")))
        })
        .collect::<io::Result<Vec<_>>>()?;

    let mut db = MetadataDb::new(output).map_err(|e| io::Error::other(format!("打开元数据数据库失败 {output}: {e}")))?;
    db.replace_all(&metadata)
        .map_err(|e| io::Error::other(format!("刷新元数据失败 {output}: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::*;

    fn metadata(code: &str, name: &str) -> Metadata {
        Metadata {
            exchange: "SSE".to_string(),
            name: Arc::from(name),
            code: Arc::from(code),
            prov: "上海".to_string(),
            city: "上海".to_string(),
            SW1: "行业一".to_string(),
            SW2: "行业二".to_string(),
            SW3: "行业三".to_string(),
            indice: HashSet::from_iter(vec!["测试指数".to_string()]),
            listing_date: "2020-01-01".to_string(),
        }
    }

    // 测试刷新过程中任意记录写入失败时，删除操作和已写记录会一起回滚。
    #[test]
    fn replace_all_rolls_back_when_insert_fails() {
        let directory = tempdir().unwrap();
        let database_path = directory.path().join("metadata.db");
        let database = Connection::open(&database_path).unwrap();
        database
            .execute_batch(
                r#"
                CREATE TABLE metadata (
                    code TEXT PRIMARY KEY,
                    exchange TEXT NOT NULL,
                    name TEXT NOT NULL CHECK(name <> 'bad'),
                    prov TEXT NOT NULL,
                    city TEXT NOT NULL,
                    sw1 TEXT NOT NULL,
                    sw2 TEXT NOT NULL,
                    sw3 TEXT NOT NULL,
                    indice TEXT NOT NULL,
                    listing_date TEXT NOT NULL
                );
                "#,
            )
            .unwrap();
        drop(database);

        let mut db = MetadataDb::new(&database_path).unwrap();
        db.add_data(&metadata("old", "old name")).unwrap();

        assert!(db.replace_all(&[metadata("new", "bad")]).is_err());
        assert!(db.query("new").unwrap().is_none());
        assert_eq!(db.query("old").unwrap().unwrap().name.as_ref(), "old name");
    }
}
