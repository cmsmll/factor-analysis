//! Mode1列表数据管理
use std::{path::Path, sync::Arc};

use serde::{Deserialize, Serialize, Serializer, ser::SerializeSeq};
use serde_json::value::RawValue;
use tokio::{
    sync::{RwLock, broadcast::Receiver},
    task::JoinSet,
};

use crate::{args::Filter, cache::Cache};

#[derive(Debug, Serialize, Deserialize)]
pub struct ListItem {
    pub args: Arc<RawValue>,
    pub data: Arc<RawValue>,
}

type Mode1Fn = Arc<dyn Fn(&Filter) -> (Arc<RawValue>, Receiver<Arc<RawValue>>) + Send + Sync + 'static>;

pub struct Mode1Manager {
    inner: RwLock<Vec<Mode1Fn>>,
    pub cache: Cache,
    pub _check_cache: Cache,
}

impl Mode1Manager {
    pub fn new(base: &Path) -> Self {
        Self {
            inner: RwLock::new(Vec::new()),
            cache: Cache::sub(base, "mode1").expect("创建 mode1 缓存目录失败"),
            _check_cache: Cache::sub(base, "mode1-check").expect("创建 mode1-check 缓存目录失败"),
        }
    }
}

impl Mode1Manager {
    pub async fn register(&self, func: Mode1Fn) {
        self.inner.write().await.push(func);
    }

    pub async fn execute(&self, filter: &Filter) -> Vec<ListItem> {
        let funcs: Vec<Mode1Fn> = self.inner.read().await.iter().map(Arc::clone).collect();
        let filter = Arc::new(filter.clone());
        let mut tasks = JoinSet::new();

        for func in funcs {
            let filter = Arc::clone(&filter);
            tasks.spawn(async move {
                let (args, mut recv) = func(&filter);
                ListItem {
                    args,
                    data: recv.recv().await.unwrap(),
                }
            });
        }
        tasks.join_all().await
    }
}

// ─── Mode1Data（分位数据容器）───────────────────────────────────────────────

use salvo_oapi::ToSchema;
use time::Date;

use crate::math::{avg_array, avg_iter};
use crate::model::Profit;

/// 分位数据
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Mode1Data {
    pub name: String,
    pub info: String,
    pub label: String,
    pub count: usize,
    pub factor: Vec<Vec<f64>>,
    pub turnover_rate: Vec<Vec<f64>>,
    pub profit1: Vec<Profit>,
    pub profit2: Vec<Profit>,
    pub profit3: Vec<Profit>,
    pub profit4: Vec<Profit>,
    #[serde(serialize_with = "serialize_datetime")]
    pub datetime: Vec<Date>,
}

/// 每个股票当期数据
pub struct Mode1Temp<'a> {
    pub factor: f64,
    pub profit: &'a [f64; 5],
}

fn serialize_datetime<S>(datetime: &[Date], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(datetime.len()))?;
    for date in datetime {
        seq.serialize_element(&date.to_string())?;
    }
    seq.end()
}

impl Mode1Data {
    pub fn new(name: impl Into<String>, info: impl Into<String>, label: impl Into<String>, count: usize) -> Self {
        assert!(count > 0, "分位数量必须大于 0");
        Self {
            name: name.into(),
            info: info.into(),
            label: label.into(),
            count,
            factor: vec![Vec::new(); count],
            turnover_rate: vec![Vec::new(); count],
            profit1: vec![Profit::new(); count],
            profit2: vec![Profit::new(); count],
            profit3: vec![Profit::new(); count],
            profit4: vec![Profit::new(); count],
            datetime: Vec::new(),
        }
    }

    pub fn push(&mut self, datetime: Date, items: &mut [Mode1Temp<'_>]) {
        if items.is_empty() {
            return;
        }
        items.sort_unstable_by(|left, right| left.factor.total_cmp(&right.factor));
        let len = items.len();
        for index in 0..self.count {
            let (start, end) = if len < self.count {
                (0, len)
            } else {
                (index * len / self.count, (index + 1) * len / self.count)
            };
            let group = unsafe { items.get_unchecked(start..end) };
            let factor = avg_iter(group.iter().map(|item| item.factor));
            let [p1, p2, p3, p4, tr] = avg_array(group.iter().map(|item| item.profit));
            self.factor[index].push(factor);
            self.turnover_rate[index].push(tr);
            self.profit1[index].push(p1);
            self.profit2[index].push(p2);
            self.profit3[index].push(p3);
            self.profit4[index].push(p4);
        }
        self.datetime.push(datetime);
    }

    pub fn raw_value(&mut self) -> Box<RawValue> {
        self.update_annualized_profit();
        let s = serde_json::to_string(self).unwrap();
        RawValue::from_string(s).unwrap()
    }

    fn update_annualized_profit(&mut self) {
        let Some(days) = self.annualized_days() else { return };
        for profits in [&mut self.profit1, &mut self.profit2, &mut self.profit3, &mut self.profit4] {
            for profit in profits {
                profit.update_annualized_profit(days);
            }
        }
    }

    fn annualized_days(&self) -> Option<f64> {
        let start = *self.datetime.first()?;
        let end = *self.datetime.last()?;
        let days = (end - start).whole_days();
        if days <= 0 { None } else { Some(days as f64) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profit(factor: f64) -> [f64; 5] {
        [factor / 100.0, factor / 10.0, -factor / 100.0, factor, factor * 10.0]
    }

    fn items<'a>(factors: &[f64], profits: &'a [[f64; 5]]) -> Vec<Mode1Temp<'a>> {
        factors
            .iter()
            .copied()
            .zip(profits)
            .map(|(factor, profit)| Mode1Temp { factor, profit })
            .collect()
    }

    #[test]
    fn raw_value_updates_annualized_profit_by_datetime_span() {
        let mut data = Mode1Data::new("测试策略", "", "测试标签", 2);
        data.datetime.push(Date::from_calendar_date(2025, time::Month::January, 1).unwrap());
        data.datetime.push(Date::from_calendar_date(2025, time::Month::January, 31).unwrap());
        data.profit1[0].push(0.1);
        data.profit2[0].push(0.2);
        let _ = data.raw_value();
        let expected1 = 0.1 / 30.0 * 365.0;
        let expected2 = 0.2 / 30.0 * 365.0;
        assert!((data.profit1[0].annualized_profit - expected1).abs() < 1e-12);
        assert!((data.profit2[0].annualized_profit - expected2).abs() < 1e-12);
        assert_eq!(data.profit3[0].annualized_profit, 0.0);
        assert_eq!(data.profit4[0].annualized_profit, 0.0);
    }

    #[test]
    fn raw_value_ignores_annualized_profit_without_datetime() {
        let mut data = Mode1Data::new("测试策略", "空时间", "测试标签", 1);
        data.profit1[0].push(0.1);
        let _ = data.raw_value();
        assert_eq!(data.profit1[0].annualized_profit, 0.0);
    }

    #[test]
    fn raw_value_serializes_datetime_as_string_array() {
        let mut data = Mode1Data::new("测试策略", "时间序列", "测试标签", 1);
        data.datetime.push(Date::from_calendar_date(2025, time::Month::January, 1).unwrap());
        data.datetime.push(Date::from_calendar_date(2025, time::Month::January, 2).unwrap());

        let raw = data.raw_value();
        let value: serde_json::Value = serde_json::from_str(raw.get()).unwrap();

        assert_eq!(value["datetime"], serde_json::json!(["2025-01-01", "2025-01-02"]));
    }

    #[test]
    fn mode1data_new_initializes_groups() {
        let data = Mode1Data::new("价值策略", "按因子从低到高分组", "基础科目及衍生类因子", 3);
        assert_eq!(data.name, "价值策略");
        assert_eq!(data.info, "按因子从低到高分组");
        assert_eq!(data.label, "基础科目及衍生类因子");
        assert_eq!(data.count, 3);
        assert_eq!(data.factor.len(), 3);
        assert_eq!(data.turnover_rate.len(), 3);
        assert_eq!(data.profit1.len(), 3);
        assert_eq!(data.profit2.len(), 3);
        assert_eq!(data.profit3.len(), 3);
        assert_eq!(data.profit4.len(), 3);
        assert!(data.datetime.is_empty());
        assert!(data.factor.iter().all(Vec::is_empty));
        assert!(data.turnover_rate.iter().all(Vec::is_empty));
    }

    #[test]
    fn mode1data_push_sorts_and_splits_items() {
        let mut data = Mode1Data::new("测试策略", "两分位", "测试标签", 2);
        let factors = [4.0, 1.0, 3.0, 2.0];
        let profits = factors.map(profit);
        let mut items = items(&factors, &profits);
        data.push(Date::from_calendar_date(2025, time::Month::January, 1).unwrap(), &mut items);
        assert_eq!(items.iter().map(|item| item.factor).collect::<Vec<_>>(), [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(data.datetime[0].to_string(), "2025-01-01");
        assert_eq!(data.factor, [vec![1.5], vec![3.5]]);
        assert_eq!(data.turnover_rate, [vec![15.0], vec![35.0]]);
        assert!((data.profit1[0].source[0] - 0.015).abs() < 1e-12);
        assert!((data.profit1[1].source[0] - 0.035).abs() < 1e-12);
    }

    #[test]
    fn mode1data_push_uses_integer_boundaries() {
        let mut data = Mode1Data::new("测试策略", "三分位", "测试标签", 3);
        let factors = [5.0, 1.0, 4.0, 2.0, 3.0];
        let profits = factors.map(profit);
        let mut items = items(&factors, &profits);
        data.push(Date::from_calendar_date(2025, time::Month::January, 2).unwrap(), &mut items);
        assert_eq!(data.factor, [vec![1.0], vec![2.5], vec![4.5]]);
    }

    #[test]
    fn mode1data_shares_items_when_count_is_insufficient() {
        let mut data = Mode1Data::new("测试策略", "四分位", "测试标签", 4);
        let factors = [3.0, 1.0];
        let profits = factors.map(profit);
        let mut items = items(&factors, &profits);
        data.push(Date::from_calendar_date(2025, time::Month::January, 3).unwrap(), &mut items);
        assert_eq!(data.factor, [vec![2.0], vec![2.0], vec![2.0], vec![2.0]]);
        assert_eq!(data.turnover_rate, [vec![20.0], vec![20.0], vec![20.0], vec![20.0]]);
        assert!(data.profit1.iter().all(|p| (p.source[0] - 0.02).abs() < 1e-12));
        assert_eq!(data.datetime[0].to_string(), "2025-01-03");
    }

    #[test]
    fn mode1data_ignores_empty_items() {
        let mut data = Mode1Data::new("测试策略", "空数据", "测试标签", 3);
        let mut items: Vec<Mode1Temp<'_>> = Vec::new();
        data.push(Date::from_calendar_date(2025, time::Month::January, 4).unwrap(), &mut items);
        assert!(data.datetime.is_empty());
        assert!(data.factor.iter().all(Vec::is_empty));
        assert!(data.profit1.iter().all(|p| p.source.is_empty()));
    }
}
