use crate::math::dev;

use super::EMA;

/// 三重指数平滑平均线变化率。
#[derive(Clone)]
pub struct TRIX {
    first: EMA,
    second: EMA,
    third: EMA,
    previous: Option<f64>,
}

impl TRIX {
    pub fn new(period: usize) -> Self {
        assert!(period >= 2, "TRIX 周期必须大于等于 2");

        Self {
            first: EMA::new(period),
            second: EMA::new(period),
            third: EMA::new(period),
            previous: None,
        }
    }

    /// 三层 EMA 预热且存在前一日 MTR 后返回 TRIX 百分比。
    pub fn next(&mut self, close: f64) -> Option<f64> {
        let first = self.first.next(close)?;
        let second = self.second.next(first)?;
        let mtr = self.third.next(second)?;
        let previous = self.previous.replace(mtr)?;

        Some(dev(mtr - previous, previous) * 100.0)
    }
}

impl Default for TRIX {
    fn default() -> Self {
        Self::new(10)
    }
}

#[cfg(test)]
mod tests {
    use super::TRIX;

    #[test]
    fn trix_returns_zero_for_constant_prices_after_warmup() {
        let mut trix = TRIX::new(2);

        for _ in 0..4 {
            assert_eq!(trix.next(10.0), None);
        }

        assert_eq!(trix.next(10.0), Some(0.0));
    }
}
