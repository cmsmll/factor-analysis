use super::EMA;

#[derive(Clone)]
pub struct MACD {
    short: EMA,
    long: EMA,
    mid: EMA,
}

impl MACD {
    pub fn new(short: usize, long: usize, mid: usize) -> Self {
        assert!(short >= 2, "MACD 短线周期必须大于等于 2");
        assert!(long >= 2, "MACD 长线周期必须大于等于 2");
        assert!(mid >= 2, "MACD 中线周期必须大于等于 2");
        assert!(short < long, "MACD 短线周期必须小于长线周期");

        Self {
            short: EMA::new(short),
            long: EMA::new(long),
            mid: EMA::new(mid),
        }
    }

    /// 预热未完成时返回 `None`，返回值为 `2 * (DIF - DEA)`。
    pub fn next(&mut self, close: f64) -> Option<f64> {
        let short = self.short.next(close);
        let long = self.long.next(close);
        let dif = short.zip(long).map(|(short, long)| short - long)?;
        let dea = self.mid.next(dif)?;

        Some(2.0 * (dif - dea))
    }
}

#[cfg(test)]
mod tests {
    use super::MACD;

    #[test]
    fn macd_returns_histogram_after_all_ema_stages_are_warm() {
        let mut macd = MACD::new(2, 3, 2);

        assert_eq!(macd.next(1.0), None);
        assert_eq!(macd.next(2.0), None);
        assert_eq!(macd.next(3.0), None);
        let value = macd.next(5.0).unwrap();

        assert!((value - 1.0 / 6.0).abs() < 1e-12);
    }
}
