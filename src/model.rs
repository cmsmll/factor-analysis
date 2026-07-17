use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::math::avg_iter;

/// 收益信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profit {
    /// 用于计算收益的数据源。
    pub source: Vec<f64>,
    /// 总收益。
    pub total_profit: f64,
    /// 总净值。
    pub total_net_value: f64,
    /// 年化收益。
    pub annualized_profit: f64,
}

impl Profit {
    const PERIODS_PER_YEAR: f64 = 365.0;

    /// 创建收益信息，其他统计值使用初始状态。
    pub fn new() -> Self {
        Self {
            source: Vec::new(),
            total_profit: 0.0,
            total_net_value: 1.0,
            annualized_profit: 0.0,
        }
    }

    /// 追加一期收益率，并更新累计收益、净值和年化收益。
    pub fn push(&mut self, profit: f64) {
        self.source.push(profit);
        self.total_net_value *= 1.0 + profit;
        self.total_profit += profit;
        let periods = self.source.len() as f64;
        self.annualized_profit = self.total_profit / periods * Self::PERIODS_PER_YEAR;
    }
}

impl Default for Profit {
    fn default() -> Self {
        Self::new()
    }
}

/// 分位数据
#[derive(Debug, Serialize, Deserialize)]
pub struct QuantileData {
    pub name: String,            // 策略名称
    pub info: String,            // 描述信息
    pub count: usize,            // 分位数量
    pub factor: Vec<Vec<f64>>,   // 因子值
    pub profit1: Vec<Profit>,    // 收益模式1: 当天收盘价买隔天收盘价卖
    pub profit2: Vec<Profit>,    // 收益模式2: 隔天开盘价买隔天收盘价卖
    pub profit3: Vec<Profit>,    // 收益模式3: 隔天开盘价买第三天开盘价卖
    pub profit4: Vec<Profit>,    // 收益模式4: 隔天开盘价买第三天收盘价卖
    pub datetime: Vec<Arc<str>>, // 日期时间
}

/// 每个股票当期数据
pub struct Item {
    pub name: Arc<str>, // 股票名称
    pub code: Arc<str>, // 股票代码
    pub factor: f64,    // 因子值
    pub profit1: f64,   // 收益模式1: 当天收盘价买隔天收盘价卖
    pub profit2: f64,   // 收益模式2: 隔天开盘价买隔天收盘价卖
    pub profit3: f64,   // 收益模式3: 隔天开盘价买第三天开盘价卖
    pub profit4: f64,   // 收益模式4: 隔天开盘价买第三天收盘价卖
}

impl QuantileData {
    /// 创建指定分位数量的数据容器。
    pub fn new(name: impl Into<String>, info: impl Into<String>, count: usize) -> Self {
        assert!(count > 0, "分位数量必须大于 0");

        Self {
            name: name.into(),
            info: info.into(),
            count,
            factor: vec![Vec::new(); count],
            profit1: vec![Profit::new(); count],
            profit2: vec![Profit::new(); count],
            profit3: vec![Profit::new(); count],
            profit4: vec![Profit::new(); count],
            datetime: Vec::new(),
        }
    }

    /// 按因子值排序并切分分位，追加各分位的平均因子和收益。
    pub fn push(&mut self, datetime: Arc<str>, mut items: Vec<Item>) {
        items.sort_by(|left, right| left.factor.total_cmp(&right.factor));

        let groups: Vec<&[Item]> = if items.len() < self.count {
            vec![items.as_slice(); self.count]
        } else {
            let len = items.len();
            let count = self.count;
            (0..count).map(|i| &items[i * len / count..(i + 1) * len / count]).collect()
        };

        for (index, group) in groups.into_iter().enumerate() {
            self.factor[index].push(avg_iter(group.iter().map(|item| item.factor)));
            self.profit1[index].push(avg_iter(group.iter().map(|item| item.profit1)));
            self.profit2[index].push(avg_iter(group.iter().map(|item| item.profit2)));
            self.profit3[index].push(avg_iter(group.iter().map(|item| item.profit3)));
            self.profit4[index].push(avg_iter(group.iter().map(|item| item.profit4)));
        }

        self.datetime.push(datetime);
    }

    pub fn raw_value(&self) -> Box<RawValue> {
        let s = serde_json::to_string(self).unwrap();
        RawValue::from_string(s).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(code: &str, factor: f64) -> Item {
        Item {
            name: Arc::from(format!("股票{code}")),
            code: Arc::from(code),
            factor,
            profit1: factor / 100.0,
            profit2: factor / 10.0,
            profit3: -factor / 100.0,
            profit4: factor,
        }
    }

    // 测试 new 保存策略信息，并按分位数量初始化所有数据容器。
    #[test]
    fn quantile_new_initializes_groups() {
        let data = QuantileData::new("价值策略", "按因子从低到高分组", 3);

        assert_eq!(data.name, "价值策略");
        assert_eq!(data.info, "按因子从低到高分组");
        assert_eq!(data.count, 3);
        assert_eq!(data.factor.len(), 3);
        assert_eq!(data.profit1.len(), 3);
        assert_eq!(data.profit2.len(), 3);
        assert_eq!(data.profit3.len(), 3);
        assert_eq!(data.profit4.len(), 3);
        assert!(data.datetime.is_empty());
        assert!(data.factor.iter().all(Vec::is_empty));
    }

    // 测试 push 取得 Item 所有权，按因子排序后切分并追加平均数据。
    #[test]
    fn quantile_push_sorts_and_splits_items() {
        let mut data = QuantileData::new("测试策略", "两分位", 2);

        data.push(
            Arc::from("2025-01-01"),
            vec![item("000004", 4.0), item("000001", 1.0), item("000003", 3.0), item("000002", 2.0)],
        );

        assert_eq!(data.datetime[0].as_ref(), "2025-01-01");
        assert_eq!(data.factor, [vec![1.5], vec![3.5]]);
        assert!((data.profit1[0].source[0] - 0.015).abs() < 1e-12);
        assert!((data.profit1[1].source[0] - 0.035).abs() < 1e-12);
        assert!((data.profit2[0].source[0] - 0.15).abs() < 1e-12);
        assert!((data.profit3[1].source[0] + 0.035).abs() < 1e-12);
        assert_eq!(data.profit4[0].source, [1.5]);
        assert_eq!(data.profit4[1].source, [3.5]);
    }

    // 测试数量不能整除时按照整数边界公式切分所有数据。
    #[test]
    fn quantile_push_uses_integer_boundaries() {
        let mut data = QuantileData::new("测试策略", "三分位", 3);

        data.push(
            Arc::from("2025-01-02"),
            vec![
                item("000005", 5.0),
                item("000001", 1.0),
                item("000004", 4.0),
                item("000002", 2.0),
                item("000003", 3.0),
            ],
        );

        assert_eq!(data.factor, [vec![1.0], vec![2.5], vec![4.5]]);
    }

    // 测试股票数量少于分位数量时，所有分位共享完整数据。
    #[test]
    fn quantile_push_shares_items_when_count_is_insufficient() {
        let mut data = QuantileData::new("测试策略", "四分位", 4);

        data.push(Arc::from("2025-01-03"), vec![item("000002", 3.0), item("000001", 1.0)]);

        assert_eq!(data.factor, [vec![2.0], vec![2.0], vec![2.0], vec![2.0]]);
        assert!(data.profit1.iter().all(|profit| (profit.source[0] - 0.02).abs() < 1e-12));
        assert_eq!(data.datetime[0].as_ref(), "2025-01-03");
    }
}
