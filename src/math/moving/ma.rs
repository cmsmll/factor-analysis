#[derive(Clone)]
pub struct SMA {
    idx: usize,
    len: usize,
    sum: f64,
    buf: Vec<f64>,
}

impl SMA {
    pub fn new(len: usize) -> Self {
        assert!(len >= 2, "SMA 周期 len 必须大于等于 2");

        Self {
            idx: 0,
            len,
            sum: 0.0,
            buf: vec![0.0; len],
        }
    }

    /// 预热未完成时返回 `None`。
    pub fn value(&self) -> Option<f64> {
        (self.idx >= self.len).then_some(self.sum / self.len as f64)
    }

    /// 预热未完成时返回 `None`。
    pub fn next(&mut self, price: f64) -> Option<f64> {
        let pos = self.idx % self.len;
        let old = self.buf[pos];
        self.buf[pos] = price;
        self.sum += price - old;
        self.idx += 1;

        self.value()
    }
}

#[derive(Clone)]
pub struct WMA {
    idx: usize,
    len: usize,
    buf: Vec<f64>,
}

impl WMA {
    pub fn new(len: usize) -> Self {
        assert!(len >= 2, "WMA 周期 len 必须大于等于 2");

        Self {
            idx: 0,
            len,
            buf: vec![0.0; len],
        }
    }

    /// 预热未完成时返回 `None`。
    pub fn value(&self) -> Option<f64> {
        if self.idx < self.len {
            return None;
        }

        let mut weight_sum = 0.0;
        let mut value_sum = 0.0;

        for i in 0..self.len {
            let weight = (i + 1) as f64;
            let pos = (self.idx + i) % self.len;

            value_sum += self.buf[pos] * weight;
            weight_sum += weight;
        }

        Some(value_sum / weight_sum)
    }

    /// 预热未完成时返回 `None`。
    pub fn next(&mut self, price: f64) -> Option<f64> {
        self.buf[self.idx % self.len] = price;
        self.idx += 1;

        self.value()
    }
}

#[derive(Clone)]
pub struct EMA {
    idx: usize,
    len: usize,
    ema: f64,
    k: f64,
}

impl EMA {
    pub fn new(len: usize) -> Self {
        assert!(len >= 2, "EMA 周期 len 必须大于等于 2");

        Self {
            idx: 0,
            len,
            ema: 0.0,
            k: 2.0 / (len as f64 + 1.0),
        }
    }

    /// 预热未完成时返回 `None`。
    pub fn value(&self) -> Option<f64> {
        (self.idx >= self.len).then_some(self.ema)
    }

    /// 预热未完成时返回 `None`。
    pub fn next(&mut self, price: f64) -> Option<f64> {
        self.idx += 1;

        if self.idx <= self.len {
            let count = self.idx as f64;
            self.ema = (self.ema * (count - 1.0) + price) / count;
        } else {
            self.ema = price * self.k + self.ema * (1.0 - self.k);
        }

        self.value()
    }
}

#[cfg(test)]
mod tests {
    use super::{EMA, SMA, WMA};

    #[test]
    fn sma_uses_runtime_window() {
        let mut sma = SMA::new(3);

        assert_eq!(sma.value(), None);
        assert_eq!(sma.next(1.0), None);
        assert_eq!(sma.value(), None);
        assert_eq!(sma.next(2.0), None);
        assert_eq!(sma.value(), None);
        assert_eq!(sma.next(3.0), Some(2.0));
        assert_eq!(sma.value(), Some(2.0));
        assert_eq!(sma.next(4.0), Some(3.0));
        assert_eq!(sma.value(), Some(3.0));
    }

    #[test]
    fn wma_weights_values_from_old_to_new() {
        let mut wma = WMA::new(3);

        assert_eq!(wma.value(), None);
        assert_eq!(wma.next(1.0), None);
        assert_eq!(wma.value(), None);
        assert_eq!(wma.next(2.0), None);
        assert_eq!(wma.value(), None);
        assert_eq!(wma.next(3.0), Some(14.0 / 6.0));
        assert_eq!(wma.value(), Some(14.0 / 6.0));
        assert_eq!(wma.next(4.0), Some(20.0 / 6.0));
        assert_eq!(wma.value(), Some(20.0 / 6.0));
    }

    #[test]
    fn ema_uses_sma_before_recursive_phase() {
        let mut ema = EMA::new(3);

        assert_eq!(ema.value(), None);
        assert_eq!(ema.next(1.0), None);
        assert_eq!(ema.value(), None);
        assert_eq!(ema.next(2.0), None);
        assert_eq!(ema.value(), None);
        assert_eq!(ema.next(3.0), Some(2.0));
        assert_eq!(ema.value(), Some(2.0));
        assert_eq!(ema.next(4.0), Some(3.0));
        assert_eq!(ema.value(), Some(3.0));
    }

    #[test]
    #[should_panic(expected = "SMA 周期 len 必须大于等于 2")]
    fn sma_rejects_short_window() {
        SMA::new(1);
    }
}
