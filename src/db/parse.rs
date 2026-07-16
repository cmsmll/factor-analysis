use std::{
    collections::BTreeSet,
    fs::File,
    io::{self, Read},
    mem,
    path::Path,
};

use encoding_rs::GBK;

pub struct ParseTbf {
    left: Symbol,
    right: Symbol,
    flag: bool,
}

impl ParseTbf {
    pub fn new(left: &str, right: &str) -> Self {
        Self {
            left: Symbol::new(left.as_bytes().to_vec()),
            right: Symbol::new(right.as_bytes().to_vec()),
            flag: false,
        }
    }

    pub fn update(&mut self, b: u8) -> bool {
        let left = self.left.update(b);
        let right = self.right.update(b);
        if left {
            self.flag = true;
        }
        if right {
            self.flag = false;
        }

        left || right
    }

    /// 解析TBF数据
    /// TBF数据可能重复使用 BTreeSet 去重并排序
    pub fn parse<P: AsRef<Path>>(&mut self, path: P) -> io::Result<BTreeSet<String>> {
        let mut file = File::open(path)?;
        let mut temp = Vec::new();
        let mut buf = [0; 1 << 10];
        let mut res = BTreeSet::new();

        while let Ok(n) = file.read(&mut buf) {
            if n == 0 {
                break;
            }
            for &b in &buf[0..n] {
                if self.update(b) {
                    // 匹配到边界但是内容小于边界
                    if temp.len() < self.right.data.len() {
                        temp.truncate(0);
                        continue;
                    }
                    let mut vec = mem::take(&mut temp);
                    // 去掉边界符
                    vec.truncate(vec.len() - self.right.data.len() + 1);
                    match String::from_utf8(vec) {
                        Ok(s) => {
                            res.insert(s);
                        }
                        Err(err) => {
                            let (s, _, ok) = GBK.decode(err.as_bytes());
                            if !ok {
                                res.insert(s.to_string());
                            }
                        }
                    }
                } else if self.flag {
                    temp.push(b);
                }
            }
        }
        Ok(res)
    }
}

pub struct Symbol {
    data: Vec<u8>,
    index: usize,
}

impl Symbol {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, index: 0 }
    }

    pub fn update(&mut self, b: u8) -> bool {
        if self.data[self.index] == b {
            self.index += 1;
            if self.index == self.data.len() {
                self.index = 0;
                return true;
            }
        } else {
            self.index = 0;
        }
        false
    }
}
