use super::SMA;

/// 多空指标，取四条简单移动均线的算术平均值。
#[derive(Clone)]
pub struct BBI {
    averages: [SMA; 4],
}

impl BBI {
    pub fn new(n1: usize, n2: usize, n3: usize, n4: usize) -> Self {
        assert!([n1, n2, n3, n4].into_iter().all(|period| period >= 2), "BBI 周期必须大于等于 2");

        Self {
            averages: [SMA::new(n1), SMA::new(n2), SMA::new(n3), SMA::new(n4)],
        }
    }

    /// 所有移动均线预热完成后返回 BBI。
    pub fn next(&mut self, close: f64) -> Option<f64> {
        let mut sum = 0.0;
        let mut ready = true;

        // 每条均线都必须接收当日收盘价，不能在某条未预热时提前返回。
        for average in &mut self.averages {
            match average.next(close) {
                Some(value) => sum += value,
                None => ready = false,
            }
        }

        ready.then_some(sum / self.averages.len() as f64)
    }
}

impl Default for BBI {
    fn default() -> Self {
        Self::new(3, 6, 12, 24)
    }
}

#[cfg(test)]
mod tests {
    use super::BBI;

    #[test]
    fn bbi_averages_all_moving_average_values() {
        let mut bbi = BBI::new(2, 3, 4, 5);

        for close in 1..5 {
            assert_eq!(bbi.next(close as f64), None);
        }

        assert_eq!(bbi.next(5.0), Some(3.75));
    }
}
