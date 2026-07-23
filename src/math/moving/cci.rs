#[derive(Clone)]
pub struct CCI {
    idx: usize,
    len: usize,
    sum: f64,
    buf: Vec<f64>,
}

impl CCI {
    pub fn new(len: usize) -> Self {
        assert!(len >= 2, "CCI 周期 len 必须大于等于 2");

        Self {
            idx: 0,
            len,
            sum: 0.0,
            buf: vec![0.0; len],
        }
    }

    pub fn next(&mut self, high: f64, low: f64, close: f64) -> Option<f64> {
        let typ = (high + low + close) / 3.0;
        let pos = self.idx % self.len;
        let old = self.buf[pos];
        self.buf[pos] = typ;
        self.sum += typ - old;
        self.idx += 1;

        if self.idx < self.len {
            return None;
        }

        let ma = self.sum / self.len as f64;
        let avedev = self.buf.iter().map(|value| (value - ma).abs()).sum::<f64>() / self.len as f64;

        Some(crate::math::dev(typ - ma, 0.015 * avedev))
    }
}

#[cfg(test)]
mod tests {
    use super::CCI;

    #[test]
    fn cci_uses_typ_average_and_avedev() {
        let mut cci = CCI::new(3);

        assert_eq!(cci.next(2.0, 0.0, 1.0), None);
        assert_eq!(cci.next(3.0, 1.0, 2.0), None);
        let value = cci.next(4.0, 2.0, 3.0).unwrap();

        assert!((value - 100.0).abs() < 1e-12);
    }
}
