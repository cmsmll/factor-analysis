use salvo_oapi::ToSchema;
use serde::{Deserialize, Serialize};


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

    pub(crate) fn update_annualized_profit(&mut self, days: f64) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // Profit::push 只更新累计收益和净值，年化收益延迟到统一计算。
    #[test]
    fn profit_push_defers_annualized_profit() {
        let mut profit = Profit::new();

        profit.push(0.1);

        assert_eq!(profit.source, [0.1]);
        assert_eq!(profit.total_profit, 0.1);
        assert_eq!(profit.total_net_value, 1.1);
        assert_eq!(profit.annualized_profit, 0.0);
    }
}
