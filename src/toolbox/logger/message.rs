use std::{
    io::{self, Write},
    sync::Arc,
    time::Duration,
};

use time::{OffsetDateTime, format_description::BorrowedFormatItem, macros::format_description};

/// 重置
const RESET: &str = "\x1b[0m";
/// 红色字体
const RED: &str = "\x1b[31m";
/// 绿色字体
const GREEN: &str = "\x1b[32m";
/// 黄色字体
const YELLOW: &str = "\x1b[33m";
/// 蓝色字体
const BLUE: &str = "\x1b[34m";

/// 红色背景
const BG_RED: &str = "\x1b[41m";
/// 绿色背景
const BG_GREEN: &str = "\x1b[42m";
/// 黄色背景
const BG_YELLOW: &str = "\x1b[43m";
/// 蓝色背景
const BG_BLUE: &str = "\x1b[44m";
/// 紫色背景
const BG_PURPLE: &str = "\x1b[45m";

const FMT: &[BorrowedFormatItem<'_>] = format_description!("[year repr:last_two]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:4]");

pub struct Message {
    pub begin: OffsetDateTime,
    pub elapsed: Duration,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub ip: String,
    pub error: Arc<str>,
    pub other: Arc<str>,
}

impl Message {
    fn status_color(&self) -> &'static str {
        match self.status {
            0..200 => BG_BLUE,
            200..300 => BG_GREEN,
            300..400 => BG_YELLOW,
            400..600 => BG_RED,
            _ => BG_PURPLE,
        }
    }

    fn elapsed_color(&self) -> &'static str {
        match self.elapsed.as_millis() {
            0..=9 => GREEN,
            10..=19 => BLUE,
            20..=29 => YELLOW,
            _ => RED,
        }
    }

    fn method_color(&self) -> &'static str {
        match self.method.as_str() {
            "GET" => BG_GREEN,
            "POST" => BG_BLUE,
            "PATCH" | "PUT" => BG_YELLOW,
            "DELETE" => BG_RED,
            _ => BG_PURPLE,
        }
    }

    fn format(&self) -> String {
        match self.begin.format(FMT) {
            Ok(s) => s,
            Err(_) => format!("{}", self.begin),
        }
    }

    pub fn write(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, "[{}] ", self.format())?;
        write!(out, "SALVO │ ")?;
        write!(out, "{} │ ", self.status)?;
        write!(out, "{:>5}ms │ ", self.elapsed.as_millis())?;
        write!(out, "{:<15} │ ", self.ip)?;
        write!(out, "{:>6} │ ", self.method)?;
        write!(out, "{} ", self.path)?;
        if !self.other.is_empty() {
            write!(out, "\n{}", self.other)?;
        }
        if !self.error.is_empty() {
            write!(out, "\n{}", self.error)?;
        }
        writeln!(out)
    }

    pub fn write_color(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, "[{}] ", self.format())?; // 时间
        write!(out, "{YELLOW}SALVO{RESET} │ ")?; // LOGO
        write!(out, "{} {} {RESET} │ ", self.status_color(), self.status)?; // 状态吗
        write!(out, "{}{:>5}ms{RESET} │ ", self.elapsed_color(), self.elapsed.as_millis())?; // 耗时
        write!(out, "{YELLOW}{:<15}{RESET} │ ", self.ip)?; // ip地址
        write!(out, "{} {:>6} {RESET} ", self.method_color(), self.method)?; // 访问方式
        write!(out, "{} ", self.path)?; // 访问路径
        if !self.other.is_empty() {
            write!(out, "\n{YELLOW}{}{RESET}", self.other)?; // 其他信息
        }
        if !self.error.is_empty() {
            write!(out, "\n{RED}{}{RESET}", self.error)?; // 错误信息
        }
        writeln!(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn message(other: &str, error: &str) -> Message {
        Message {
            begin: OffsetDateTime::UNIX_EPOCH,
            elapsed: Duration::default(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            status: 200,
            ip: "127.0.0.1".to_string(),
            error: Arc::from(error),
            other: Arc::from(other),
        }
    }

    // 测试附加文本为空时，普通日志只输出主日志行和一个结束换行。
    #[test]
    fn write_skips_empty_text_lines() {
        let mut output = Vec::new();
        message("", "").write(&mut output).unwrap();

        assert_eq!(output.iter().filter(|&&byte| byte == b'\n').count(), 1);
    }

    // 测试附加文本为空时，彩色日志只输出主日志行和一个结束换行。
    #[test]
    fn write_color_skips_empty_text_lines() {
        let mut output = Vec::new();
        message("", "").write_color(&mut output).unwrap();

        assert_eq!(output.iter().filter(|&&byte| byte == b'\n').count(), 1);
    }

    // 测试非空附加文本仍各自占用一行，不产生额外空行。
    #[test]
    fn write_keeps_non_empty_text_lines() {
        let mut output = Vec::new();
        message("other", "error").write(&mut output).unwrap();

        assert_eq!(output.iter().filter(|&&byte| byte == b'\n').count(), 3);
    }
}
