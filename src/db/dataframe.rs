use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use itertools::izip;
use salvo::Depot;
use time::Date;

use crate::db::{Finance, MarketData, market::MarketDataDb, metadata::Metadata};

#[derive(Debug)]
pub struct DataFrame {
    pub end: String,
    pub start: String,
    /// 股票状态数据
    pub depot: Vec<Depot>,
    /// 数据列表
    pub list: Vec<Contract>,
    /// 索引表
    pub index: Vec<Arc<str>>,
    /// 元数据
    pub meta: Arc<Vec<Metadata>>,
}

impl DataFrame {
    pub fn init_depot<T>(&mut self, f: impl Fn(&mut Depot) -> T) {
        self.depot.clear();
        for _ in 0..self.list.len() {
            let mut depot = Depot::new();
            f(&mut depot);
            self.depot.push(depot);
        }
    }

    pub fn list_zip_depot(&mut self) -> impl Iterator<Item = (&Contract, &mut Depot)> {
        self.list.iter().zip(self.depot.iter_mut())
    }

    pub fn list_zip_depot_meta(
        &mut self,
    ) -> impl Iterator<Item = (&Contract, &mut Depot, &Metadata)> {
        izip!(self.list.iter(), self.depot.iter_mut(), self.meta.iter())
    }
}

pub struct DataFrameBuild {
    pub start: Date,
    pub end: Date,
    pub data: PathBuf,
    pub info: PathBuf,
    pub category: PathBuf,
}

impl DataFrameBuild {
    pub fn load(&self) -> DataFrame {
        let s = fs::read_to_string(&self.category).unwrap();
        let category: Vec<String> = serde_json::from_str(&s).unwrap();
        let data_path = Path::new(&self.data);
        let mut result: Vec<Contract> = Vec::with_capacity(category.len());
        let mut index_table: BTreeSet<Arc<str>> = BTreeSet::new();

        for code in category {
            let database_path = data_path.join(format!("{code}.db"));
            let db = MarketDataDb::new(&database_path).unwrap();
            let (item, table) = db
                .query_with_table(self.start, self.end, &mut index_table)
                .expect(&format!("获取数据库失败{:?}", database_path));

            if !item.is_empty() {
                result.push(Contract {
                    start: item.first().unwrap().datetime.clone(),
                    end: item.last().unwrap().datetime.clone(),
                    data: item,
                    table,
                    finance: Vec::default(),
                });
            }
        }

        let start = index_table
            .first()
            .map(|datetime| datetime.to_string())
            .unwrap_or_default();
        let end = index_table
            .last()
            .map(|datetime| datetime.to_string())
            .unwrap_or_default();

        DataFrame {
            start,
            end,
            list: result,
            index: index_table.into_iter().collect(),
            meta: Default::default(),
            depot: Vec::default(),
        }
    }
}

/// 时间索引
pub struct Index {
    pub index: usize,
    pub datetime: Arc<str>,
}

impl Index {
    pub fn new(index: usize, datetime: impl Into<Arc<str>>) -> Self {
        Self {
            index,
            datetime: datetime.into(),
        }
    }
}

/// 合约数据
#[derive(Debug)]
pub struct Contract {
    pub start: Arc<str>,
    pub end: Arc<str>,
    /// 行情数据
    pub data: Vec<Arc<MarketData>>,
    /// 时间表
    pub table: HashMap<Arc<str>, usize>,
    /// 财务数据
    pub finance: Vec<Arc<Finance>>,
}

impl Contract {
    pub fn index(&self, index: &Index) -> Option<usize> {
        self.table.get(&index.datetime).copied()
    }

    pub fn data(&self, i: &Index) -> Option<(Arc<MarketData>, usize)> {
        let index = self.index(i)?;
        Some((self.data.get(index)?.clone(), index))
    }

    pub fn before(&self, index: &Index, n: usize) -> Option<Arc<MarketData>> {
        let i = self.index(index)?;
        self.data.get(i.checked_sub(n)?).map(Arc::clone)
    }

    pub fn after(&self, index: &Index, n: usize) -> Option<Arc<MarketData>> {
        let i = self.index(index)?;
        self.data.get(i.checked_add(n)?).map(Arc::clone)
    }
}
