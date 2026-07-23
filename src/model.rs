
use salvo_oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use time::Date;

use crate::math::{avg_array, avg_iter};

/// 收益信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
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

    /// 追加一期收益率，并更新累计收益和净值。
    pub fn push(&mut self, profit: f64) {
        self.source.push(profit);
        self.total_net_value *= 1.0 + profit;
        self.total_profit += profit;
    }

    fn update_annualized_profit(&mut self, days: f64) {
        if self.source.is_empty() || days <= 0.0 {
            return;
        }

        self.annualized_profit = self.total_profit / days * Self::PERIODS_PER_YEAR;
    }
}

impl Default for Profit {
    fn default() -> Self {
        Self::new()
    }
}

/// 分位数据
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QuantileData {
    pub name: String,                 // 策略名称
    pub info: String,                 // 描述信息
    pub count: usize,                 // 分位数量
    pub factor: Vec<Vec<f64>>,        // 因子值
    pub turnover_rate: Vec<Vec<f64>>, // 各分位平均换手率
    pub profit1: Vec<Profit>,         // 收益模式1: 当天收盘价买隔天收盘价卖
    pub profit2: Vec<Profit>,         // 收益模式2: 隔天开盘价买隔天收盘价卖
    pub profit3: Vec<Profit>,         // 收益模式3: 隔天开盘价买第三天开盘价卖
    pub profit4: Vec<Profit>,         // 收益模式4: 隔天开盘价买第三天收盘价卖
    pub datetime: Vec<Date>,      // 日期时间
}

/// 每个股票当期数据
pub struct TempItem<'a> {
    pub factor: f64,
    pub profit: &'a [f64; 5],
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
            turnover_rate: vec![Vec::new(); count],
            profit1: vec![Profit::new(); count],
            profit2: vec![Profit::new(); count],
            profit3: vec![Profit::new(); count],
            profit4: vec![Profit::new(); count],
            datetime: Vec::new(),
        }
    }

    /// 按因子值排序并切分分位，追加各分位的平均因子、平均换手率和收益。
    pub fn push(&mut self, datetime: Date, items: &mut [TempItem<'_>]) {
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
            let [profit1, profit2, profit3, profit4, turnover_rate] = avg_array(group.iter().map(|item| item.profit));

            self.factor[index].push(factor);
            self.turnover_rate[index].push(turnover_rate);
            self.profit1[index].push(profit1);
            self.profit2[index].push(profit2);
            self.profit3[index].push(profit3);
            self.profit4[index].push(profit4);
        }

        self.datetime.push(datetime);
    }

    pub fn raw_value(&mut self) -> Box<RawValue> {
        self.update_annualized_profit();
        let s = serde_json::to_string(self).unwrap();
        RawValue::from_string(s).unwrap()
    }

    fn update_annualized_profit(&mut self) {
        let Some(days) = self.annualized_days() else {
            return;
        };

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

        (days > 0).then_some(days as f64)
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    fn profit(factor: f64) -> [f64; 5] {
        [factor / 100.0, factor / 10.0, -factor / 100.0, factor, factor * 10.0]
    }

    fn items<'a>(factors: &[f64], profits: &'a [[f64; 5]]) -> Vec<TempItem<'a>> {
        factors
            .iter()
            .copied()
            .zip(profits)
            .map(|(factor, profit)| TempItem { factor, profit })
            .collect()
    }

    // 测试 Profit::push 只更新累计收益和净值，年化收益延迟到 QuantileData::raw_value 统一计算。
    #[test]
    fn profit_push_defers_annualized_profit() {
        let mut profit = Profit::new();

        profit.push(0.1);

        assert_eq!(profit.source, [0.1]);
        assert_eq!(profit.total_profit, 0.1);
        assert_eq!(profit.total_net_value, 1.1);
        assert_eq!(profit.annualized_profit, 0.0);
    }

    // 测试 raw_value 根据 datetime 首尾日期跨度统一刷新所有收益模式的年化收益。
    #[test]
    fn raw_value_updates_annualized_profit_by_datetime_span() {
        let mut data = QuantileData::new("测试策略", "年化", 1);
        data.datetime = vec![Date::from_calendar_date(2025, time::Month::January, 1).unwrap(), Date::from_calendar_date(2025, time::Month::January, 31).unwrap()];
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

    // 测试 datetime 没有数据时 raw_value 不处理年化收益。
    #[test]
    fn raw_value_ignores_annualized_profit_without_datetime() {
        let mut data = QuantileData::new("测试策略", "空时间", 1);
        data.profit1[0].push(0.1);

        let _ = data.raw_value();

        assert_eq!(data.profit1[0].annualized_profit, 0.0);
    }

    // 测试 new 保存策略信息，并按分位数量初始化所有数据容器。
    #[test]
    fn quantile_new_initializes_groups() {
        let data = QuantileData::new("价值策略", "按因子从低到高分组", 3);

        assert_eq!(data.name, "价值策略");
        assert_eq!(data.info, "按因子从低到高分组");
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

    // 测试 push 原地排序可变切片，按因子切分并追加平均数据。
    #[test]
    fn quantile_push_sorts_and_splits_items() {
        let mut data = QuantileData::new("测试策略", "两分位", 2);
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
        assert!((data.profit2[0].source[0] - 0.15).abs() < 1e-12);
        assert!((data.profit3[1].source[0] + 0.035).abs() < 1e-12);
        assert_eq!(data.profit4[0].source, [1.5]);
        assert_eq!(data.profit4[1].source, [3.5]);
    }

    // 测试数量不能整除时按照整数边界公式切分所有数据。
    #[test]
    fn quantile_push_uses_integer_boundaries() {
        let mut data = QuantileData::new("测试策略", "三分位", 3);
        let factors = [5.0, 1.0, 4.0, 2.0, 3.0];
        let profits = factors.map(profit);
        let mut items = items(&factors, &profits);

        data.push(Date::from_calendar_date(2025, time::Month::January, 2).unwrap(), &mut items);

        assert_eq!(data.factor, [vec![1.0], vec![2.5], vec![4.5]]);
    }

    // 测试股票数量少于分位数量时，所有分位共享完整数据。
    #[test]
    fn quantile_push_shares_items_when_count_is_insufficient() {
        let mut data = QuantileData::new("测试策略", "四分位", 4);
        let factors = [3.0, 1.0];
        let profits = factors.map(profit);
        let mut items = items(&factors, &profits);

        data.push(Date::from_calendar_date(2025, time::Month::January, 3).unwrap(), &mut items);

        assert_eq!(data.factor, [vec![2.0], vec![2.0], vec![2.0], vec![2.0]]);
        assert_eq!(data.turnover_rate, [vec![20.0], vec![20.0], vec![20.0], vec![20.0]]);
        assert!(data.profit1.iter().all(|profit| (profit.source[0] - 0.02).abs() < 1e-12));
        assert_eq!(data.datetime[0].to_string(), "2025-01-03");
    }

    // 测试空股票集合不会写入日期或虚假的零值。
    #[test]
    fn quantile_push_ignores_empty_items() {
        let mut data = QuantileData::new("测试策略", "空数据", 3);
        let mut items: Vec<TempItem<'_>> = Vec::new();

        data.push(Date::from_calendar_date(2025, time::Month::January, 4).unwrap(), &mut items);

        assert!(data.datetime.is_empty());
        assert!(data.factor.iter().all(Vec::is_empty));
        assert!(data.profit1.iter().all(|profit| profit.source.is_empty()));
    }
}
