use crate::math::dev;

use super::SMA;

/// MASS 主线及其 M 日移动平均信号线。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MassValue {
    pub mass: f64,
    pub ma_mass: Option<f64>,
}

/// 梅斯线指标。
#[derive(Clone)]
pub struct MASS {
    range_average: SMA,
    range_average_twice: SMA,
    mass_average: SMA,
    signal: SMA,
    n2: f64,
}

impl MASS {
    pub fn new(n1: usize, n2: usize, m: usize) -> Self {
        assert!([n1, n2, m].into_iter().all(|period| period >= 2), "MASS 周期必须大于等于 2");

        Self {
            range_average: SMA::new(n1),
            range_average_twice: SMA::new(n1),
            mass_average: SMA::new(n2),
            signal: SMA::new(m),
            n2: n2 as f64,
        }
    }

    /// MASS 主线预热完成后返回结果，信号线尚未预热时 `ma_mass` 为 `None`。
    pub fn next(&mut self, high: f64, low: f64) -> Option<MassValue> {
        let range_average = self.range_average.next(high - low)?;
        let range_average_twice = self.range_average_twice.next(range_average)?;
        let ratio = dev(range_average, range_average_twice);
        let mass = self.mass_average.next(ratio)? * self.n2;
        let ma_mass = self.signal.next(mass);

        Some(MassValue { mass, ma_mass })
    }
}

impl Default for MASS {
    fn default() -> Self {
        Self::new(9, 25, 6)
    }
}

#[cfg(test)]
mod tests {
    use super::MASS;

    #[test]
    fn mass_returns_main_and_signal_lines() {
        let mut mass = MASS::new(2, 2, 2);

        assert_eq!(mass.next(1.0, 0.0), None);
        assert_eq!(mass.next(2.0, 0.0), None);
        assert_eq!(mass.next(3.0, 0.0), None);

        let first = mass.next(4.0, 0.0).unwrap();
        assert!((first.mass - 29.0 / 12.0).abs() < 1e-12);
        assert_eq!(first.ma_mass, None);

        let second = mass.next(5.0, 0.0).unwrap();
        assert!((second.mass - 55.0 / 24.0).abs() < 1e-12);
        assert!((second.ma_mass.unwrap() - 113.0 / 48.0).abs() < 1e-12);
    }
}
