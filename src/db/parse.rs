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
        self.left.index = 0;
        self.right.index = 0;
        self.flag = false;
        let mut file = File::open(path)?;
        let mut temp = Vec::new();
        let mut buf = [0; 1 << 10];
        let mut res = BTreeSet::new();

        loop {
            let n = file.read(&mut buf)?;
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
                            // 有些异常数据不解析
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
        if self.flag {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "TBF 数据缺少结束边界"));
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
        assert!(!data.is_empty(), "TBF 边界符不能为空");
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

#[cfg(test)]
mod tests {
    use std::{fs, io};

    use tempfile::tempdir;

    use super::*;

    // 测试文件在记录中途结束时返回 UnexpectedEof，避免把不完整数据写入数据库。
    #[test]
    fn parse_rejects_unclosed_record() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("broken.tbf");
        fs::write(&path, b"<begin>{\"value\":1}").unwrap();

        let error = ParseTbf::new("<begin>", "</end>").parse(path).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
    }

    // 测试完整的多条记录可以被提取、去重并排序。
    #[test]
    fn parse_reads_complete_records() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("valid.tbf");
        fs::write(&path, b"prefix<begin>b</end><begin>a</end><begin>b</end>").unwrap();

        let data = ParseTbf::new("<begin>", "</end>").parse(path).unwrap();

        assert_eq!(data.into_iter().collect::<Vec<_>>(), ["a", "b"]);
    }
}
