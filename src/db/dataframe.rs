use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use time::Date;

use crate::{
    args::Filter,
    db::{Finance, MarketData, Metadata},
};

#[derive(Debug)]
pub struct DataFrame {
    pub end: Date,
    pub start: Date,
    /// 索引表
    pub index: Vec<Arc<str>>,
    /// 数据列表
    pub list: Vec<Arc<Contract>>,
    /// 板块列表
    pub sector: Arc<HashSet<String>>,
    /// 指数列表
    pub indice: Arc<HashSet<String>>,
}

impl DataFrame {
    /// 按索引表顺序迭代时间索引。
    pub fn index_iter(&self) -> impl Iterator<Item = Index> + '_ {
        self.index.iter().enumerate().map(|(index, datetime)| Index::new(index, datetime.clone()))
    }

    /// 返回指定日期范围内的新数据帧，超出的边界会被裁剪。
    pub fn range(&self, start: Date, end: Date) -> Self {
        let start = start.max(self.start);
        let end = end.min(self.end);
        let start_text = start.to_string();
        let end_text = end.to_string();
        let index = self
            .index
            .iter()
            .filter(|datetime| datetime.as_ref() >= start_text.as_str() && datetime.as_ref() <= end_text.as_str())
            .cloned()
            .collect();

        Self {
            start,
            end,
            index,
            list: self.list.clone(),
            sector: self.sector.clone(),
            indice: self.indice.clone(),
        }
    }

    /// 返回指定日期范围内并按条件过滤合约的新数据帧。
    ///
    /// 过滤闭包返回 `true` 时保留该合约，返回 `false` 时移除该合约。
    pub fn range_filter<F>(&self, start: Date, end: Date, mut filter: F) -> Self
    where
        F: FnMut(&Arc<Contract>) -> bool,
    {
        let mut frame = self.range(start, end);
        frame.list.retain(|contract| filter(contract));
        frame
    }

    /// 根据参数裁剪日期并过滤合约，板块和指数条件使用并集。
    pub fn filter(&self, args: &Filter) -> Self {
        let has_metadata_filter = !args.sector.is_empty() || !args.indice.is_empty();

        self.range_filter(args.start, args.end, |contract| {
            let metadata = &contract.metadata;
            if args.filter_bz && metadata.exchange == "北京证券交易所" {
                return false;
            }
            if args.filter_st && metadata.name.contains("ST") {
                return false;
            }

            if !has_metadata_filter {
                return true;
            }

            args.sector.contains(&metadata.SW1)
                || args.sector.contains(&metadata.SW2)
                || args.sector.contains(&metadata.SW3)
                || metadata.indice.iter().any(|indice| args.indice.contains(indice))
        })
    }
}

pub(super) fn collect_metadata_lists(list: &[Arc<Contract>]) -> (Arc<HashSet<String>>, Arc<HashSet<String>>) {
    let mut sector = HashSet::new();
    let mut indice = HashSet::new();

    for contract in list {
        let metadata = &contract.metadata;
        sector.extend(
            [&metadata.SW1, &metadata.SW2, &metadata.SW3]
                .into_iter()
                .filter(|value| !value.is_empty())
                .cloned(),
        );
        indice.extend(metadata.indice.iter().filter(|value| !value.is_empty()).cloned());
    }

    (Arc::new(sector), Arc::new(indice))
}

/// 时间索引
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// 合约元数据
    pub metadata: Metadata,
    /// 时间表
    pub table: HashMap<Arc<str>, usize>,
    /// 行情数据
    pub market: Vec<Arc<MarketData>>,
    /// 财务数据
    pub finance: Vec<Arc<Finance>>,
}

impl Contract {
    pub fn index(&self, index: &Index) -> Option<usize> {
        self.table.get(&index.datetime).copied()
    }

    pub fn data(&self, i: &Index) -> Option<(&MarketData, usize)> {
        let index = self.index(i)?;
        Some((self.market.get(index)?, index))
    }

    pub fn before(&self, index: &Index, n: usize) -> Option<&MarketData> {
        let i = self.index(index)?;
        self.market.get(i.checked_sub(n)?).map(Arc::as_ref)
    }

    pub fn after(&self, index: &Index, n: usize) -> Option<&MarketData> {
        let i = self.index(index)?;
        self.market.get(i.checked_add(n)?).map(Arc::as_ref)
    }
}
#[cfg(test)]
mod tests {
    use time::{Date, Month};

    use super::*;

    fn date(day: u8) -> Date {
        Date::from_calendar_date(2025, Month::January, day).unwrap()
    }

    fn contract(code: &str, exchange: &str, sector: &str, indice: &str) -> Arc<Contract> {
        Arc::new(Contract {
            start: Arc::from("2025-01-01"),
            end: Arc::from("2025-01-03"),
            metadata: Metadata {
                exchange: exchange.to_string(),
                name: Arc::from(code),
                code: Arc::from(code),
                prov: String::new(),
                city: String::new(),
                SW1: sector.to_string(),
                SW2: String::new(),
                SW3: String::new(),
                indice: HashSet::from([indice.to_string()]),
                listing_date: "2020-01-01".to_string(),
            },
            table: HashMap::new(),
            market: Vec::new(),
            finance: Vec::new(),
        })
    }

    fn frame() -> DataFrame {
        let list = vec![
            contract("000001", "上海证券交易所", "行业一", "沪深指数"),
            contract("830001", "北京证券交易所", "行业二", "北证指数"),
        ];
        let (sector, indice) = collect_metadata_lists(&list);

        DataFrame {
            start: date(1),
            end: date(3),
            index: vec![Arc::from("2025-01-01"), Arc::from("2025-01-02"), Arc::from("2025-01-03")],
            list,
            sector,
            indice,
        }
    }

    fn args(sector: HashSet<String>, indice: HashSet<String>, filter_bz: bool, filter_st: bool) -> Filter {
        Filter {
            start: date(2),
            end: date(4),
            filter_bz,
            filter_st,
            sector,
            indice,
        }
    }

    // 测试板块和指数条件使用并集，任意一项匹配即可保留合约。
    #[test]
    fn from_args_uses_sector_and_indice_union() {
        let frame = frame();
        let filtered = frame.filter(&args(
            HashSet::from(["行业一".to_string()]),
            HashSet::from(["北证指数".to_string()]),
            false,
            false,
        ));

        assert_eq!(filtered.start, date(2));
        assert_eq!(filtered.end, date(3));
        assert_eq!(filtered.index.len(), 2);
        assert_eq!(filtered.list.len(), 2);
        assert!(Arc::ptr_eq(&filtered.sector, &frame.sector));
        assert!(Arc::ptr_eq(&filtered.indice, &frame.indice));
    }

    // 测试 filter_bz 只排除北京证券交易所。
    #[test]
    fn from_args_filters_beijing_exchange_only() {
        let frame = frame();
        let filtered = frame.filter(&args(HashSet::new(), HashSet::new(), true, false));

        assert_eq!(filtered.list.len(), 1);
        assert_eq!(filtered.list[0].metadata.exchange, "上海证券交易所");
        assert_eq!(filtered.sector.len(), 2);
        assert_eq!(filtered.indice.len(), 2);
    }

    // 测试 filter_st 会排除名称中包含 ST 的合约。
    #[test]
    fn from_args_filters_st_contracts() {
        let mut frame = frame();
        frame.list.push(contract("ST0001", "上海证券交易所", "行业三", "测试指数"));

        let filtered = frame.filter(&args(HashSet::new(), HashSet::new(), false, true));

        assert_eq!(filtered.list.len(), 2);
        assert!(filtered.list.iter().all(|contract| !contract.metadata.name.contains("ST")));
    }

    // 测试板块和指数均不匹配时过滤全部合约，但不修改原始列表信息。
    #[test]
    fn from_args_keeps_metadata_lists_when_no_contract_matches() {
        let frame = frame();
        let filtered = frame.filter(&args(
            HashSet::from(["不存在的板块".to_string()]),
            HashSet::from(["不存在的指数".to_string()]),
            false,
            false,
        ));

        assert!(filtered.list.is_empty());
        assert!(Arc::ptr_eq(&filtered.sector, &frame.sector));
        assert!(Arc::ptr_eq(&filtered.indice, &frame.indice));
    }
}
