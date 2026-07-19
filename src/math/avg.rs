use super::sum_simd;

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

/// 单次遍历计算五项收益指标的平均值。
pub fn avg_array<'a>(iter: impl IntoIterator<Item = &'a [f64; 5]>) -> [f64; 5] {
    let mut sums = [0.0; 5];
    let mut len = 0usize;

    for values in iter {
        len += 1;
        for (sum, value) in sums.iter_mut().zip(values) {
            *sum += *value;
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

/// 使用可用的 SIMD 指令计算平均值，空切片返回零。
pub fn avg_simd(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        sum_simd(values) / values.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::{avg_array, avg_iter, avg_simd};

    // 测试标量平均值函数保持原有行为。
    #[test]
    fn avg_iter_calculates_average() {
        assert_eq!(avg_iter([1.0, 2.0, 3.0]), 2.0);
        assert_eq!(avg_iter([]), 0.0);
    }

    // 测试单次遍历可以同时计算多项指标的平均值。
    #[test]
    fn avg_array_calculates_all_columns() {
        let values = [[1.0, 3.0, 5.0, 7.0, 9.0], [3.0, 5.0, 7.0, 9.0, 11.0]];

        assert_eq!(avg_array(&values), [2.0, 4.0, 6.0, 8.0, 10.0]);
        assert_eq!(avg_array(std::iter::empty()), [0.0; 5]);
    }

    // 测试 SIMD 平均值并处理空切片。
    #[test]
    fn avg_simd_calculates_average() {
        assert_eq!(avg_simd(&[1.0, 2.0, 3.0, 4.0]), 2.5);
        assert_eq!(avg_simd(&[]), 0.0);
    }
}
