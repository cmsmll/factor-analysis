pub mod dataframe;
pub mod finance;
pub mod market;
pub mod metadata;
pub mod parse;

use std::{collections::BTreeSet, path::Path, sync::Arc};

pub use dataframe::*;
pub use finance::*;
pub use market::*;
pub use metadata::*;
use rayon::prelude::*;

use rusqlite::{Connection, OpenFlags, Result, params, types::Type};
use time::{Date, format_description::well_known::Iso8601};

use crate::config::Config;

pub struct DataFrameDb {
    /// 行情数据库，与 metadata 使用相同顺序。
    pub market: Vec<Connection>,
    /// 财务数据库，与 metadata 使用相同顺序。
    pub finance: Vec<Connection>,
    /// 合约信息数据库。
    pub metadata: Arc<Vec<Metadata>>,
}

impl DataFrameDb {
    pub fn new<D, F, M>(market_path: D, finance_path: F, metadata_path: M) -> Result<Self>
    where
        D: AsRef<Path>,
        F: AsRef<Path>,
        M: AsRef<Path>,
    {
        let finance_path = finance_path.as_ref();
        let market_path = market_path.as_ref();
        let metadata_db = MetadataDb::open_read_only(metadata_path)?;
        let metadata = Arc::new(metadata_db.query_all()?);

        let market = open_contract_databases(&metadata, market_path)?;
        let finance = open_contract_databases(&metadata, finance_path)?;

        Ok(Self { market, finance, metadata })
    }

    /// 根据配置中的行情、财务和元数据路径打开数据库。
    pub fn from_config(config: &Config) -> Result<Self> {
        let data = &config.data;
        Self::new(&data.market, &data.finance, &data.metadata)
    }

    pub fn query(&self, start: Date, end: Date) -> Result<DataFrame> {
        let start = start.to_string();
        let end = end.saturating_add(time::Duration::days(1)).to_string();
        self.build(Some((&start, &end)))
    }

    /// 查询全部行情，并为每条行情关联不晚于自身时间的最近一期财务数据。
    pub fn query_all(&self) -> Result<DataFrame> {
        self.build(None)
    }

    fn build(&self, range: Option<(&str, &str)>) -> Result<DataFrame> {
        let mut list = Vec::new();
        let mut index_table = BTreeSet::new();

        for ((market_conn, finance_conn), metadata) in self.market.iter().zip(self.finance.iter()).zip(self.metadata.iter()) {
            let market = query_market(market_conn, range, &mut index_table)?;
            let Some((first, last)) = market.first().zip(market.last()) else {
                continue;
            };
            let finance = query_finance(finance_conn, range, &index_table)?;
            align_check(&market, &finance)?;
            let table = market.iter().enumerate().map(|(index, md)| (md.datetime.clone(), index)).collect();
            let start = first.datetime.clone();
            let end = last.datetime.clone();

            list.push(Arc::new(Contract {
                start,
                end,
                metadata: metadata.clone(),
                table,
                market,
                finance,
            }));
        }

        let start = index_table.first().ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let end = index_table.last().ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let start = parse_index_date(start)?;
        let end = parse_index_date(end)?;
        let (sector, indice) = dataframe::collect_metadata_lists(&list);

        Ok(DataFrame {
            start,
            end,
            list,
            index: index_table.into_iter().collect(),
            sector,
            indice,
        })
    }
}

fn parse_index_date(value: &str) -> Result<Date> {
    Date::parse(value, &Iso8601::DATE).map_err(|error| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error)))
}

fn open_contract_databases(metadata: &[Metadata], directory: &Path) -> Result<Vec<Connection>> {
    metadata
        .par_iter()
        .map(|metadata| {
            let path = directory.join(format!("{}.db", metadata.code));
            Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        })
        .collect()
}

fn query_market(database: &Connection, range: Option<(&str, &str)>, index_table: &mut BTreeSet<Arc<str>>) -> Result<Vec<Arc<MarketData>>> {
    let sql = if range.is_some() {
        include_str!("sql/market_query_range.sql")
    } else {
        include_str!("sql/market_query_all.sql")
    };
    let mut stmt = database.prepare(sql)?;
    let map_row = |row: &rusqlite::Row<'_>| {
        let mut datetime: Arc<str> = Arc::from(row.get::<_, String>(0)?);
        if let Some(shared) = index_table.get(datetime.as_ref()) {
            datetime = shared.clone();
        } else {
            index_table.insert(datetime.clone());
        }
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
    };

    match range {
        Some((start, end)) => stmt.query_map(params![start, end], map_row)?.collect(),
        None => stmt.query_map([], map_row)?.collect(),
    }
}

fn query_finance(database: &Connection, range: Option<(&str, &str)>, index_table: &BTreeSet<Arc<str>>) -> Result<Vec<Arc<Finance>>> {
    let sql = if range.is_some() {
        include_str!("sql/finance_query_range.sql")
    } else {
        include_str!("sql/finance_query_all.sql")
    };
    let mut stmt = database.prepare(sql)?;
    let map_row = |row: &rusqlite::Row<'_>| {
        let datetime = row.get::<_, String>(0)?;
        let datetime = index_table.get(datetime.as_str()).cloned().unwrap_or_else(|| Arc::from(datetime));

        Ok(Arc::new(Finance {
            datetime,
            total_shares: row.get(1)?,
            float_shares: row.get(2)?,
            total_market: row.get(3)?,
            float_market: row.get(4)?,
        }))
    };

    match range {
        Some((start, end)) => stmt.query_map(params![start, end], map_row)?.collect(),
        None => stmt.query_map([], map_row)?.collect(),
    }
}

fn align_check(market: &[Arc<MarketData>], finance: &[Arc<Finance>]) -> Result<()> {
    if market.len() != finance.len() || !market.iter().zip(finance).all(|(market, finance)| market.datetime == finance.datetime) {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }

    Ok(())
}
#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc};

    use tempfile::tempdir;
    use time::{Date, Month};

    use super::*;

    fn date(day: u8) -> Date {
        Date::from_calendar_date(2025, Month::January, day).unwrap()
    }

    fn metadata(code: &str) -> Metadata {
        Metadata {
            exchange: "SSE".to_string(),
            name: Arc::from(format!("测试{code}")),
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

    fn finance(datetime: &str, total_shares: f64) -> Finance {
        Finance {
            datetime: Arc::from(datetime),
            total_shares,
            float_shares: total_shares / 2.0,
            total_market: total_shares * 10.0,
            float_market: total_shares * 5.0,
        }
    }

    // 测试行情与财务按相同顺序精确对齐，并复用相同的日期 Arc。
    #[test]
    fn query_checks_market_and_finance_alignment() {
        let directory = tempdir().unwrap();
        let metadata_path = directory.path().join("metadata.db");
        let market_dir = directory.path().join("market");
        let finance_dir = directory.path().join("finance");
        std::fs::create_dir_all(&market_dir).unwrap();
        std::fs::create_dir_all(&finance_dir).unwrap();
        let market_path = market_dir.join("000001.db");
        let finance_path = finance_dir.join("000001.db");

        {
            let mut db = MetadataDb::new(&metadata_path).unwrap();
            db.replace_all(&[metadata("000001")]).unwrap();
        }
        {
            let mut db = MarketDataDb::new(&market_path).unwrap();
            db.replace_all(&[market("2025-01-01", 10.0), market("2025-01-02", 11.0), market("2025-01-03", 12.0)])
                .unwrap();
        }
        {
            let mut db = FinanceDB::new(&finance_path).unwrap();
            db.replace_all(&[finance("2025-01-01", 80.0), finance("2025-01-02", 100.0), finance("2025-01-03", 120.0)])
                .unwrap();
        }

        let market_all = MarketDataDb::open_read_only(&market_path).unwrap().query_all().unwrap();
        let finance_all = FinanceDB::open_read_only(&finance_path).unwrap().query_all().unwrap();
        let config = Config {
            server: Default::default(),
            period: Vec::new(),
            data: crate::config::DataConfig {
                market: market_dir.clone(),
                finance: finance_dir.clone(),
                metadata: metadata_path.clone(),
                ..Default::default()
            },
        };
        let db = DataFrameDb::from_config(&config).unwrap();
        let all_frame = db.query_all().unwrap();
        let frame = db.query(date(1), date(3)).unwrap();
        let range_frame = db.query(date(2), date(3)).unwrap();

        assert_eq!(market_all.len(), 3);
        assert_eq!(market_all[0].datetime.as_ref(), "2025-01-01");
        assert_eq!(market_all[2].datetime.as_ref(), "2025-01-03");
        assert_eq!(finance_all.len(), 3);
        assert_eq!(finance_all[0].datetime.as_ref(), "2025-01-01");
        assert_eq!(all_frame.start, date(1));
        assert_eq!(all_frame.end, date(3));

        // 测试板块和指数列表从合约元数据汇总并去重。
        assert_eq!(all_frame.sector.len(), 3);
        assert!(all_frame.sector.contains("行业一"));
        assert!(all_frame.sector.contains("行业二"));
        assert!(all_frame.sector.contains("行业三"));
        assert_eq!(all_frame.indice.len(), 1);
        assert!(all_frame.indice.contains("测试指数"));

        // 测试 index_iter 按索引表顺序返回位置和日期，并复用日期 Arc。
        let indices = all_frame.index_iter().collect::<Vec<_>>();
        assert_eq!(indices.iter().map(|item| item.index).collect::<Vec<_>>(), [0, 1, 2]);
        assert_eq!(
            indices.iter().map(|item| item.datetime.as_ref()).collect::<Vec<_>>(),
            ["2025-01-01", "2025-01-02", "2025-01-03"]
        );
        assert!(Arc::ptr_eq(&indices[0].datetime, &all_frame.index[0]));

        assert_eq!(all_frame.list[0].market.len(), 3);
        assert_eq!(all_frame.list[0].finance[0].datetime.as_ref(), "2025-01-01");
        assert_eq!(all_frame.list[0].finance[1].datetime.as_ref(), "2025-01-02");
        assert_eq!(frame.list.len(), 1);
        assert_eq!(frame.list[0].metadata.code.as_ref(), "000001");
        assert_eq!(frame.list[0].market.len(), 3);
        assert_eq!(frame.list[0].finance.len(), 3);
        assert_eq!(frame.list[0].finance[0].datetime.as_ref(), "2025-01-01");
        assert_eq!(frame.list[0].finance[1].datetime.as_ref(), "2025-01-02");
        assert_eq!(range_frame.list[0].market.len(), 2);
        assert_eq!(range_frame.list[0].finance.len(), 2);
        assert_eq!(range_frame.list[0].finance[0].datetime.as_ref(), "2025-01-02");
        assert_eq!(range_frame.list[0].finance[1].datetime.as_ref(), "2025-01-03");
        for (market, finance) in range_frame.list[0].market.iter().zip(range_frame.list[0].finance.iter()) {
            assert!(Arc::ptr_eq(&market.datetime, &finance.datetime));
        }
        for (market, finance) in frame.list[0].market.iter().zip(frame.list[0].finance.iter()) {
            assert!(Arc::ptr_eq(&market.datetime, &finance.datetime));
        }

        // 测试超出 DataFrame 的请求范围会裁剪到实际边界，并返回共享数据的新 DataFrame。
        let before = Date::from_calendar_date(2024, Month::December, 31).unwrap();
        let ranged = all_frame.range(before, date(4));
        assert_eq!(ranged.start, date(1));
        assert_eq!(ranged.end, date(3));
        assert_eq!(
            ranged.index.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
            ["2025-01-01", "2025-01-02", "2025-01-03"]
        );
        assert!(Arc::ptr_eq(&ranged.index[0], &all_frame.index[0]));
        assert!(Arc::ptr_eq(&ranged.list[0], &all_frame.list[0]));
        assert!(Arc::ptr_eq(&ranged.sector, &all_frame.sector));
        assert!(Arc::ptr_eq(&ranged.indice, &all_frame.indice));

        // 测试部分范围会同步修改起止日期和索引。
        let partial = all_frame.range(date(2), date(4));
        assert_eq!(partial.start, date(2));
        assert_eq!(partial.end, date(3));
        assert_eq!(partial.index.iter().map(AsRef::as_ref).collect::<Vec<_>>(), ["2025-01-02", "2025-01-03"]);
        assert!(Arc::ptr_eq(&partial.index[0], &all_frame.index[1]));

        // 测试 range_filter 在日期裁剪基础上过滤合约，并继续复用原始 Arc。
        let filtered = all_frame.range_filter(date(2), date(4), |contract: &Arc<Contract>| contract.metadata.code.as_ref() == "000001");
        assert_eq!(filtered.start, date(2));
        assert_eq!(filtered.end, date(3));
        assert_eq!(filtered.index.iter().map(AsRef::as_ref).collect::<Vec<_>>(), ["2025-01-02", "2025-01-03"]);
        assert_eq!(filtered.list.len(), 1);
        assert!(Arc::ptr_eq(&filtered.list[0], &all_frame.list[0]));

        // 测试过滤掉全部合约时，日期范围和索引保持不变。
        let filtered_empty = all_frame.range_filter(date(2), date(3), |_| false);
        assert_eq!(filtered_empty.start, date(2));
        assert_eq!(filtered_empty.end, date(3));
        assert_eq!(filtered_empty.index.len(), 2);
        assert!(filtered_empty.list.is_empty());
        assert!(Arc::ptr_eq(&filtered_empty.sector, &all_frame.sector));
        assert!(Arc::ptr_eq(&filtered_empty.indice, &all_frame.indice));

        // 测试无交集范围和反向范围返回空索引，并保留裁剪后的边界。
        let empty_before = all_frame.range(before, before);
        assert_eq!(empty_before.start, date(1));
        assert_eq!(empty_before.end, before);
        assert!(empty_before.index.is_empty());
        let reversed = all_frame.range(date(3), date(2));
        assert_eq!(reversed.start, date(3));
        assert_eq!(reversed.end, date(2));
        assert!(reversed.index.is_empty());
    }

    // 测试财务数据无法覆盖首条行情时查询报错，不使用默认财务数据填充。
    #[test]
    fn query_rejects_market_without_matching_finance() {
        let directory = tempdir().unwrap();
        let metadata_path = directory.path().join("metadata.db");
        let market_dir = directory.path().join("market");
        let finance_dir = directory.path().join("finance");
        std::fs::create_dir_all(&market_dir).unwrap();
        std::fs::create_dir_all(&finance_dir).unwrap();
        let market_path = market_dir.join("000004.db");
        let finance_path = finance_dir.join("000004.db");

        let mut metadata_db = MetadataDb::new(&metadata_path).unwrap();
        metadata_db.replace_all(&[metadata("000004")]).unwrap();
        let mut market_db = MarketDataDb::new(&market_path).unwrap();
        market_db.replace_all(&[market("2025-01-01", 10.0)]).unwrap();
        let mut finance_db = FinanceDB::new(&finance_path).unwrap();
        finance_db.replace_all(&[finance("2025-01-02", 100.0)]).unwrap();

        let db = DataFrameDb::new(&market_dir, &finance_dir, &metadata_path).unwrap();

        assert!(db.query(date(1), date(1)).is_err());
        assert!(db.query_all().is_err());
    }
    // 测试行情数据库存在但财务数据库缺失时，构造直接报错且不创建空文件。
    #[test]
    fn new_rejects_missing_finance_database() {
        let directory = tempdir().unwrap();
        let metadata_path = directory.path().join("metadata.db");
        let market_dir = directory.path().join("market");
        let finance_dir = directory.path().join("finance");
        std::fs::create_dir_all(&market_dir).unwrap();
        let market_path = market_dir.join("000002.db");
        let missing_finance = finance_dir.join("000002.db");

        let mut metadata_db = MetadataDb::new(&metadata_path).unwrap();
        metadata_db.replace_all(&[metadata("000002")]).unwrap();
        let mut market_db = MarketDataDb::new(&market_path).unwrap();
        market_db.replace_all(&[market("2025-01-01", 10.0)]).unwrap();

        assert!(DataFrameDb::new(&market_dir, &finance_dir, &metadata_path).is_err());
        assert!(!missing_finance.exists());
    }
    // 测试只读打开缺失的合约数据库时返回错误，并且不会创建空数据库文件。
    #[test]
    fn new_does_not_create_missing_contract_database() {
        let directory = tempdir().unwrap();
        let metadata_path = directory.path().join("metadata.db");
        let market_dir = directory.path().join("market");
        let finance_dir = directory.path().join("finance");
        std::fs::create_dir_all(&market_dir).unwrap();
        let missing_market = market_dir.join("000003.db");
        let missing_finance = finance_dir.join("000003.db");

        {
            let mut db = MetadataDb::new(&metadata_path).unwrap();
            db.replace_all(&[metadata("000003")]).unwrap();
        }

        assert!(DataFrameDb::new(&market_dir, &finance_dir, &metadata_path).is_err());
        assert!(!missing_market.exists());
        assert!(!missing_finance.exists());
    }
}
