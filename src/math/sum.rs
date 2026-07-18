/// 使用可用的 SIMD 指令求和，不支持时回退到标量实现。
pub fn sum_simd(values: &[f64]) -> f64 {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if std::is_x86_feature_detected!("avx512f") {
        // SAFETY: 调用前已经确认当前 CPU 支持 AVX-512F。
        return unsafe { sum_avx512f(values) };
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if std::is_x86_feature_detected!("avx2") {
        // SAFETY: 调用前已经确认当前 CPU 支持 AVX2。
        return unsafe { sum_avx2(values) };
    }

    values.iter().sum()
}

#[cfg(target_arch = "x86")]
#[target_feature(enable = "avx512f")]
unsafe fn sum_avx512f(values: &[f64]) -> f64 {
    use std::arch::x86::{_mm512_add_pd, _mm512_loadu_pd, _mm512_setzero_pd, _mm512_storeu_pd};

    let mut sum = _mm512_setzero_pd();
    let simd_len = values.len() / 8 * 8;
    let mut index = 0;

    while index < simd_len {
        // SAFETY: index 始终位于切片内，并且 loadu 不要求内存对齐。
        let values = unsafe { _mm512_loadu_pd(values.as_ptr().add(index)) };
        sum = _mm512_add_pd(sum, values);
        index += 8;
    }

    let mut lanes = [0.0; 8];
    // SAFETY: lanes 提供了连续的八个 f64 写入空间。
    unsafe { _mm512_storeu_pd(lanes.as_mut_ptr(), sum) };

    lanes.into_iter().sum::<f64>() + values[simd_len..].iter().sum::<f64>()
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn sum_avx512f(values: &[f64]) -> f64 {
    use std::arch::x86_64::{_mm512_add_pd, _mm512_loadu_pd, _mm512_setzero_pd, _mm512_storeu_pd};

    let mut sum = _mm512_setzero_pd();
    let simd_len = values.len() / 8 * 8;
    let mut index = 0;

    while index < simd_len {
        // SAFETY: index 始终位于切片内，并且 loadu 不要求内存对齐。
        let values = unsafe { _mm512_loadu_pd(values.as_ptr().add(index)) };
        sum = _mm512_add_pd(sum, values);
        index += 8;
    }

    let mut lanes = [0.0; 8];
    // SAFETY: lanes 提供了连续的八个 f64 写入空间。
    unsafe { _mm512_storeu_pd(lanes.as_mut_ptr(), sum) };

    lanes.into_iter().sum::<f64>() + values[simd_len..].iter().sum::<f64>()
}

#[cfg(target_arch = "x86")]
#[target_feature(enable = "avx2")]
unsafe fn sum_avx2(values: &[f64]) -> f64 {
    use std::arch::x86::{_mm256_add_pd, _mm256_loadu_pd, _mm256_setzero_pd, _mm256_storeu_pd};

    let mut sum = _mm256_setzero_pd();
    let simd_len = values.len() / 4 * 4;
    let mut index = 0;

    while index < simd_len {
        // SAFETY: index 始终位于切片内，并且 loadu 不要求内存对齐。
        let values = unsafe { _mm256_loadu_pd(values.as_ptr().add(index)) };
        sum = _mm256_add_pd(sum, values);
        index += 4;
    }

    let mut lanes = [0.0; 4];
    // SAFETY: lanes 提供了连续的四个 f64 写入空间。
    unsafe { _mm256_storeu_pd(lanes.as_mut_ptr(), sum) };

    lanes.into_iter().sum::<f64>() + values[simd_len..].iter().sum::<f64>()
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn sum_avx2(values: &[f64]) -> f64 {
    use std::arch::x86_64::{_mm256_add_pd, _mm256_loadu_pd, _mm256_setzero_pd, _mm256_storeu_pd};

    let mut sum = _mm256_setzero_pd();
    let simd_len = values.len() / 4 * 4;
    let mut index = 0;

    while index < simd_len {
        // SAFETY: index 始终位于切片内，并且 loadu 不要求内存对齐。
        let values = unsafe { _mm256_loadu_pd(values.as_ptr().add(index)) };
        sum = _mm256_add_pd(sum, values);
        index += 4;
    }

    let mut lanes = [0.0; 4];
    // SAFETY: lanes 提供了连续的四个 f64 写入空间。
    unsafe { _mm256_storeu_pd(lanes.as_mut_ptr(), sum) };

    lanes.into_iter().sum::<f64>() + values[simd_len..].iter().sum::<f64>()
}

#[cfg(test)]
mod tests {
    use super::sum_simd;

    // 测试 SIMD 求和包含无法组成完整向量的尾部数据。
    #[test]
    fn sum_simd_includes_tail_values() {
        let values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];

        assert_eq!(sum_simd(&values), 55.0);
        assert_eq!(sum_simd(&[]), 0.0);
    }
}
