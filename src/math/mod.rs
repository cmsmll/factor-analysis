/// 计算单项指标的平均值。
pub fn avg_iter(iter: impl IntoIterator<Item = f64>) -> f64 {
    let mut sum = 0.0;
    let mut len = 0usize;

    for value in iter {
        sum += value;
        len += 1;
    }

    if len == 0 { 0.0 } else { sum / len as f64 }
}

/// 单次遍历计算固定数量指标的平均值。
pub fn avg_array<const N: usize>(iter: impl IntoIterator<Item = [f64; N]>) -> [f64; N] {
    let mut sums = [0.0; N];
    let mut len = 0usize;

    for values in iter {
        len += 1;
        for (sum, value) in sums.iter_mut().zip(values) {
            *sum += value;
        }
    }

    if len > 0 {
        let len = len as f64;
        for sum in &mut sums {
            *sum /= len;
        }
    }

    sums
}

/// 执行除法，分母为零时返回零。
#[inline]
pub fn dev(value: f64, divisor: f64) -> f64 {
    if divisor == 0.0 { 0.0 } else { value / divisor }
}

#[cfg(test)]
mod tests {
    use super::{avg_array, avg_iter, dev};

    // 测试标量平均值函数保持原有行为。
    #[test]
    fn avg_iter_calculates_average() {
        assert_eq!(avg_iter([1.0, 2.0, 3.0]), 2.0);
        assert_eq!(avg_iter([]), 0.0);
    }

    // 测试单次遍历可以同时计算多项指标的平均值。
    #[test]
    fn avg_array_calculates_all_columns() {
        assert_eq!(avg_array([[1.0, 3.0], [3.0, 5.0]]), [2.0, 4.0]);
        assert_eq!(avg_array::<2>([]), [0.0, 0.0]);
    }

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
