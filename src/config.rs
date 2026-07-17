use std::{
    fs,
    io::{self, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    process,
};

use serde::{Deserialize, Serialize};
use tempfile::Builder;
use time::{Date, Month};

use crate::toolbox::date_format;

/// 当前目录下的默认配置文件名。
pub const CONFIG_FILE: &str = "config.toml";

/// 应用配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// HTTP 服务器配置。
    #[serde(default)]
    pub server: ServerConfig,
    /// 分析参数配置。
    #[serde(default)]
    pub args: ArgsConfig,
    /// 数据源和数据库路径配置。
    #[serde(default)]
    pub data: DataConfig,
}

impl Config {
    /// 从当前目录加载配置；失败时打印错误并以状态码 0 退出。
    pub fn load() -> Self {
        Self::load_from(CONFIG_FILE).unwrap_or_else(|error| exit_with_error("加载配置失败", error))
    }

    /// 从指定路径加载 TOML 配置。
    pub fn load_from(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let content =
            fs::read_to_string(path).map_err(|error| io::Error::new(error.kind(), format!("读取配置文件 {} 失败: {error}", path.display())))?;
        toml::from_str(&content).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, format!("解析配置文件 {} 失败: {error}", path.display())))
    }

    /// 加载当前目录的配置；不存在时生成默认配置，其他错误打印后以状态码 0 退出。
    pub fn load_or_gen_default() -> Self {
        Self::load_or_gen_default_at(CONFIG_FILE).unwrap_or_else(|error| exit_with_error("加载或生成配置失败", error))
    }

    /// 加载指定路径的配置；文件不存在时生成并保存默认配置。
    pub fn load_or_gen_default_at(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        match Self::load_from(path) {
            Ok(config) => Ok(config),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Self::default_at(path),
            Err(error) => Err(error),
        }
    }

    /// 生成默认配置并保存到当前目录的 `config.toml`。
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> io::Result<Self> {
        Self::default_at(CONFIG_FILE)
    }

    /// 生成默认配置并保存到指定路径。
    pub fn default_at(path: impl AsRef<Path>) -> io::Result<Self> {
        let config = Self::default_values();
        config.save_to(path)?;
        Ok(config)
    }

    /// 将当前配置保存到当前目录的 `config.toml`。
    pub fn save(&self) -> io::Result<()> {
        self.save_to(CONFIG_FILE)
    }

    /// 将当前配置以格式化 TOML 原子保存到指定路径。
    pub fn save_to(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;

        let content = toml::to_string_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, format!("序列化配置文件 {} 失败: {error}", path.display())))?;
        let mut temp_file = Builder::new().suffix(".tmp").tempfile_in(parent)?;
        temp_file.write_all(content.as_bytes())?;
        temp_file.as_file().sync_all()?;
        temp_file.persist(path).map_err(|error| error.error)?;

        Ok(())
    }

    /// 返回服务器监听地址。
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.server.addr, self.server.port)
    }

    fn default_values() -> Self {
        Self {
            server: ServerConfig::default(),
            args: ArgsConfig::default(),
            data: DataConfig::default(),
        }
    }
}

fn exit_with_error(message: &str, error: io::Error) -> ! {
    eprintln!("{message}: {error}");
    process::exit(0);
}

/// HTTP 服务器配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    /// 监听地址。
    pub addr: IpAddr,
    /// 监听端口。
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 7878,
        }
    }
}

/// 分析参数配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArgsConfig {
    /// 开始日期。
    #[serde(with = "date_format")]
    pub start: Date,
    /// 结束日期。
    #[serde(with = "date_format")]
    pub end: Date,
}

impl Default for ArgsConfig {
    fn default() -> Self {
        Self {
            start: Date::from_calendar_date(2025, Month::January, 1).expect("默认开始日期有效"),
            end: Date::from_calendar_date(2025, Month::December, 31).expect("默认结束日期有效"),
        }
    }
}
/// 数据库和原始 TBF 数据路径配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DataConfig {
    /// 行情数据库目录。
    pub market: PathBuf,
    /// 财务数据库目录。
    pub finance: PathBuf,
    /// 元数据数据库文件。
    pub metadata: PathBuf,
    /// 原始行情 TBF 数据目录。
    pub tbf_market: PathBuf,
    /// 原始财务 TBF 数据目录。
    pub tbf_finance: PathBuf,
    /// 原始元数据 TBF 数据目录。
    pub tbf_metadata: PathBuf,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            market: PathBuf::from("data/database/market"),
            finance: PathBuf::from("data/database/finance"),
            metadata: PathBuf::from("data/database/metadata.db"),
            tbf_market: PathBuf::from("data/tbf/market"),
            tbf_finance: PathBuf::from("data/tbf/finance"),
            tbf_metadata: PathBuf::from("data/tbf/metadata"),
        }
    }
}
#[cfg(test)]
mod tests {
    use std::{fs, net::SocketAddr, path::Path};

    use tempfile::tempdir;

    use super::*;

    // 测试默认配置会写入 TOML 文件，并且可以无损重新加载。
    #[test]
    fn default_at_saves_and_loads_config() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(CONFIG_FILE);

        let expected = Config::default_at(&path).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let actual = Config::load_from(&path).unwrap();

        assert!(content.contains("[server]"));
        assert!(content.contains("[args]"));
        assert!(content.contains("start = \"2025-01-01\""));
        assert!(content.contains("end = \"2025-12-31\""));
        assert!(content.contains("[data]"));
        assert_eq!(actual, expected);
    }

    // 测试配置文件不存在时生成默认配置并写入指定路径。
    #[test]
    fn load_or_gen_default_at_generates_missing_config() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(CONFIG_FILE);

        let config = Config::load_or_gen_default_at(&path).unwrap();

        assert!(path.exists());
        assert_eq!(Config::load_from(path).unwrap(), config);
    }

    // 测试配置已存在时直接加载，不使用默认配置覆盖已有内容。
    #[test]
    fn load_or_gen_default_at_loads_existing_config() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(CONFIG_FILE);
        fs::write(&path, "[server]\nport = 9000\n").unwrap();

        let config = Config::load_or_gen_default_at(&path).unwrap();

        assert_eq!(config.server.port, 9000);
        assert!(fs::read_to_string(path).unwrap().contains("port = 9000"));
    }

    // 测试已有配置格式错误时返回错误，不会覆盖原文件。
    #[test]
    fn load_or_gen_default_at_keeps_invalid_config() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(CONFIG_FILE);
        fs::write(&path, "invalid toml").unwrap();

        let error = Config::load_or_gen_default_at(&path).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(fs::read_to_string(path).unwrap(), "invalid toml");
    }

    // 测试配置只填写部分字段时，其余字段自动使用默认值。
    #[test]
    fn load_fills_missing_fields_with_defaults() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(CONFIG_FILE);
        fs::write(
            &path,
            r#"
                [server]
                port = 9000

                [data]
                tbf_market = "custom/market"
            "#,
        )
        .unwrap();

        let config = Config::load_from(path).unwrap();

        assert_eq!(config.server.port, 9000);
        assert_eq!(config.server.addr, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(config.data.tbf_market, Path::new("custom/market"));
        assert_eq!(config.data.finance, DataConfig::default().finance);
        assert_eq!(config.args, ArgsConfig::default());
    }

    // 测试 args 日期字符串通过公共 date_format 转换为 Date。
    #[test]
    fn load_parses_args_dates() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(CONFIG_FILE);
        fs::write(
            &path,
            r#"
                [args]
                start = "2024-01-02"
                end = "2026-06-30"
            "#,
        )
        .unwrap();

        let config = Config::load_from(path).unwrap();

        assert_eq!(config.args.start, Date::from_calendar_date(2024, Month::January, 2).unwrap());
        assert_eq!(config.args.end, Date::from_calendar_date(2026, Month::June, 30).unwrap());
    }
    // 测试重新生成默认配置会完整替换已有文件，不会留下旧字段。
    #[test]
    fn default_at_overwrites_existing_config() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(CONFIG_FILE);
        fs::write(&path, "old = true").unwrap();

        let expected = Config::default_at(&path).unwrap();
        let actual = Config::load_from(&path).unwrap();

        assert_eq!(actual, expected);
        assert!(!fs::read_to_string(path).unwrap().contains("old"));
    }

    // 测试未知字段会返回 InvalidData，避免配置拼写错误被静默忽略。
    #[test]
    fn load_rejects_unknown_fields() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(CONFIG_FILE);
        fs::write(
            &path,
            r#"
                [server]
                prot = 9000
            "#,
        )
        .unwrap();

        let error = Config::load_from(path).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    // 测试服务器地址和端口可以组合为监听地址。
    #[test]
    fn config_builds_socket_address() {
        let config = Config::default_values();

        assert_eq!(config.socket_addr(), "127.0.0.1:7878".parse::<SocketAddr>().unwrap());
    }
}
