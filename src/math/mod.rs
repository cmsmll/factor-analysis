mod avg;
mod moving;
mod sum;

pub use avg::*;
pub use moving::*;
pub use sum::*;

/// 执行除法，分母为零时返回零。
#[inline]
pub fn dev(value: f64, divisor: f64) -> f64 {
    if divisor == 0.0 { 0.0 } else { value / divisor }
}

#[cfg(test)]
mod tests {
    use super::dev;

    // 测试正常除法返回计算结果。
    #[test]
    fn dev_divides_values() {
        assert_eq!(dev(10.0, 4.0), 2.5);
    }

    // 测试正零和负零作为分母时都直接返回零。
    #[test]
    fn dev_returns_zero_when_divisor_is_zero() {
        assert_eq!(dev(10.0, 0.0), 0.0);
        assert_eq!(dev(10.0, -0.0), 0.0);
    }
}
