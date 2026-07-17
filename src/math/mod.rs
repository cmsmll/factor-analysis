// 计算平均指标
pub fn avg_iter(iter: impl IntoIterator<Item = f64>) -> f64 {
    let mut sum = 0.0;
    let mut len = 0.0;
    for item in iter {
        len += 1.0;
        sum += item
    }
    if len == 0.0 {
        return 0.0;
    }

    sum / len
}
