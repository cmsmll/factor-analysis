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
use rusqlite::{Connection, OpenFlags, Result, params};
use time::Date;

pub struct DataFrameDb {
    /// 合约数据库，与 metadata 使用相同顺序。
    pub contract: Vec<Connection>,
    /// 合约信息数据库。
    pub metadata: Arc<Vec<Metadata>>,
}

impl DataFrameDb {
    pub fn new<D, M>(data_path: D, metadata_path: M) -> Result<Self>
    where
        D: AsRef<Path>,
        M: AsRef<Path>,
    {
        let data_path = data_path.as_ref();
        let metadata_db = MetadataDb::open_read_only(metadata_path)?;
        let metadata = Arc::new(metadata_db.query_all()?);

        let contract = metadata
            .par_iter()
            .map(|metadata| {
                let database_path = data_path.join(format!("{}.db", metadata.code));
                Connection::open_with_flags(database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { contract, metadata })
    }

    pub fn query(&self, start: Date, end: Date) -> Result<DataFrame> {
        let start_text = start.to_string();
        let end_text = end.saturating_add(time::Duration::days(1)).to_string();

        let mut list = Vec::new();
        let mut meta = Vec::new();
        let mut index_table: BTreeSet<Arc<str>> = BTreeSet::new();

        for (database, metadata) in self.contract.iter().zip(self.metadata.iter()) {
            let (data, finance) =
                query_market_finance(database, &start_text, &end_text, &mut index_table)?;

            let Some((first, last)) = data.first().zip(data.last()) else {
                continue;
            };
            let start = first.datetime.clone();
            let end = last.datetime.clone();
            let table = data
                .iter()
                .enumerate()
                .map(|(index, md)| (md.datetime.clone(), index))
                .collect();

            list.push(Contract {
                start,
                end,
                data,
                table,
                finance,
            });
            meta.push(metadata.clone());
        }

        let start = index_table
            .first()
            .map(|datetime| datetime.to_string())
            .unwrap_or_default();
        let end = index_table
            .last()
            .map(|datetime| datetime.to_string())
            .unwrap_or_default();

        Ok(DataFrame {
            start,
            end,
            depot: Vec::default(),
            list,
            index: index_table.into_iter().collect(),
            meta: Arc::new(meta),
        })
    }
}

type MarketFinanceData = (Vec<Arc<MarketData>>, Vec<Arc<Finance>>);

fn query_market_finance(
    database: &Connection,
    start: &str,
    end: &str,
    index_table: &mut BTreeSet<Arc<str>>,
) -> Result<MarketFinanceData> {
    let has_finance = database.query_row(
        include_str!("sql/table_exists.sql"),
        params!["financial"],
        |row| row.get(0),
    )?;
    let sql = if has_finance {
        include_str!("sql/dataframe_query_market_finance.sql")
    } else {
        include_str!("sql/dataframe_query_market.sql")
    };
    let mut stmt = database.prepare(sql)?;

    let mut data = Vec::new();
    let mut finance = Vec::new();
    let mut last_finance = Arc::new(Finance::default());

    let rows = stmt.query_map(params![start, end], |row| {
        let mut datetime: Arc<str> = Arc::from(row.get::<_, String>(0)?);
        if let Some(arc_dt) = index_table.get(datetime.as_ref()) {
            datetime = arc_dt.clone();
        } else {
            index_table.insert(datetime.clone());
        }

        let md = Arc::new(MarketData {
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
        });

        let financial_datetime = row.get::<_, Option<String>>(10)?;
        let financial = if let Some(financial_datetime) = financial_datetime {
            Finance {
                datetime: Arc::from(financial_datetime),
                total_shares: row.get(11)?,
                float_shares: row.get(12)?,
                total_market: row.get(13)?,
                float_market: row.get(14)?,
            }
        } else {
            Finance::default()
        };
        let financial = if last_finance.same_data(&financial) {
            last_finance.clone()
        } else {
            let financial = Arc::new(financial);
            last_finance = financial.clone();
            financial
        };

        Ok((md, financial))
    })?;

    for row in rows {
        let (md, financial) = row?;
        data.push(md);
        finance.push(financial);
    }

    Ok((data, finance))
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
            indice: vec!["测试指数".to_string()],
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

    // 测试每条行情关联不晚于自身时间的最近一期财务数据，并复用相同 Arc。
    #[test]
    fn query_uses_latest_finance_and_keeps_metadata_aligned() {
        let directory = tempdir().unwrap();
        let metadata_path = directory.path().join("metadata.db");
        let contract_path = directory.path().join("000001.db");

        {
            let mut db = MetadataDb::new(&metadata_path).unwrap();
            db.replace_all(&[metadata("000001")]).unwrap();
        }
        {
            let mut db = MarketDataDb::new(&contract_path).unwrap();
            db.replace_all(&[
                market("2025-01-01", 10.0),
                market("2025-01-02", 11.0),
                market("2025-01-03", 12.0),
            ])
            .unwrap();
        }
        {
            let mut db = FinanceDB::new(&contract_path).unwrap();
            db.replace_all(&[finance("2025-01-02", 100.0)]).unwrap();
        }

        let db = DataFrameDb::new(directory.path(), &metadata_path).unwrap();
        let frame = db.query(date(1), date(3)).unwrap();

        assert_eq!(frame.list.len(), 1);
        assert_eq!(frame.meta[0].code.as_ref(), "000001");
        assert_eq!(frame.list[0].data.len(), 3);
        assert_eq!(frame.list[0].finance.len(), 3);
        assert_eq!(frame.list[0].finance[0].datetime.as_ref(), "");
        assert_eq!(frame.list[0].finance[1].datetime.as_ref(), "2025-01-02");
        assert!(Arc::ptr_eq(
            &frame.list[0].finance[1],
            &frame.list[0].finance[2]
        ));
    }

    // 测试合约没有财务表时仍返回行情，财务列表使用共享的默认值占位。
    #[test]
    fn query_supports_contract_without_finance_table() {
        let directory = tempdir().unwrap();
        let metadata_path = directory.path().join("metadata.db");
        let contract_path = directory.path().join("000002.db");

        {
            let mut db = MetadataDb::new(&metadata_path).unwrap();
            db.replace_all(&[metadata("000002")]).unwrap();
        }
        {
            let mut db = MarketDataDb::new(&contract_path).unwrap();
            db.replace_all(&[market("2025-01-01", 10.0), market("2025-01-02", 11.0)])
                .unwrap();
        }

        let db = DataFrameDb::new(directory.path(), &metadata_path).unwrap();
        let frame = db.query(date(1), date(2)).unwrap();

        assert_eq!(frame.list[0].data.len(), 2);
        assert_eq!(frame.list[0].finance.len(), 2);
        assert!(Arc::ptr_eq(
            &frame.list[0].finance[0],
            &frame.list[0].finance[1]
        ));
    }

    // 测试只读打开缺失的合约数据库时返回错误，并且不会创建空数据库文件。
    #[test]
    fn new_does_not_create_missing_contract_database() {
        let directory = tempdir().unwrap();
        let metadata_path = directory.path().join("metadata.db");
        let missing_path = directory.path().join("000003.db");

        {
            let mut db = MetadataDb::new(&metadata_path).unwrap();
            db.replace_all(&[metadata("000003")]).unwrap();
        }

        assert!(DataFrameDb::new(directory.path(), &metadata_path).is_err());
        assert!(!missing_path.exists());
    }
}
