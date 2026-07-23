#[derive(Clone)]
pub struct High {
    idx: usize,
    len: usize,
    hit: usize,
    buf: Vec<f64>,
}

impl High {
    pub fn new(len: usize) -> Self {
        assert!(len >= 2, "High 周期 len 必须大于等于 2");

        Self {
            idx: 0,
            len,
            hit: 0,
            buf: vec![f64::NEG_INFINITY; len],
        }
    }

    /// 预热未完成时返回 `None`。
    pub fn value(&self) -> Option<f64> {
        (self.idx >= self.len).then_some(self.buf[self.hit])
    }

    /// 预热未完成时返回 `None`。
    pub fn next(&mut self, price: f64) -> Option<f64> {
        let pos = self.idx % self.len;

        if price >= self.buf[self.hit] {
            self.buf[pos] = price;
            self.hit = pos;
            self.idx += 1;
            return self.value();
        }

        self.buf[pos] = price;
        self.idx += 1;

        if self.hit == pos {
            self.hit = pos;
            let count = self.len.min(self.idx);
            let start = (self.idx + self.len - count) % self.len;

            for i in 0..count {
                let current = (start + i) % self.len;
                if self.buf[current] >= self.buf[self.hit] {
                    self.hit = current;
                }
            }
        }

        self.value()
    }
}

#[derive(Clone)]
pub struct Low {
    idx: usize,
    len: usize,
    hit: usize,
    buf: Vec<f64>,
}

impl Low {
    pub fn new(len: usize) -> Self {
        assert!(len >= 2, "Low 周期 len 必须大于等于 2");

        Self {
            idx: 0,
            len,
            hit: 0,
            buf: vec![f64::INFINITY; len],
        }
    }

    /// 预热未完成时返回 `None`。
    pub fn value(&self) -> Option<f64> {
        (self.idx >= self.len).then_some(self.buf[self.hit])
    }

    /// 预热未完成时返回 `None`。
    pub fn next(&mut self, price: f64) -> Option<f64> {
        let pos = self.idx % self.len;

        if price <= self.buf[self.hit] {
            self.buf[pos] = price;
            self.hit = pos;
            self.idx += 1;
            return self.value();
        }

        self.buf[pos] = price;
        self.idx += 1;

        if self.hit == pos {
            self.hit = pos;
            let count = self.len.min(self.idx);
            let start = (self.idx + self.len - count) % self.len;

            for i in 0..count {
                let current = (start + i) % self.len;
                if self.buf[current] <= self.buf[self.hit] {
                    self.hit = current;
                }
            }
        }

        self.value()
    }
}

#[cfg(test)]
mod tests {
    use super::{High, Low};

    #[test]
    fn high_tracks_latest_window_high() {
        let mut high = High::new(3);

        assert_eq!(high.value(), None);
        assert_eq!(high.next(1.0), None);
        assert_eq!(high.value(), None);
        assert_eq!(high.next(3.0), None);
        assert_eq!(high.value(), None);
        assert_eq!(high.next(2.0), Some(3.0));
        assert_eq!(high.value(), Some(3.0));
        assert_eq!(high.next(0.0), Some(3.0));
        assert_eq!(high.value(), Some(3.0));
        assert_eq!(high.next(1.0), Some(2.0));
        assert_eq!(high.value(), Some(2.0));
    }

    #[test]
    fn low_tracks_latest_window_low() {
        let mut low = Low::new(3);

        assert_eq!(low.value(), None);
        assert_eq!(low.next(3.0), None);
        assert_eq!(low.value(), None);
        assert_eq!(low.next(1.0), None);
        assert_eq!(low.value(), None);
        assert_eq!(low.next(2.0), Some(1.0));
        assert_eq!(low.value(), Some(1.0));
        assert_eq!(low.next(4.0), Some(1.0));
        assert_eq!(low.value(), Some(1.0));
        assert_eq!(low.next(5.0), Some(2.0));
        assert_eq!(low.value(), Some(2.0));
    }
}
