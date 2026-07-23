use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    config::Config,
    db::{FinanceDB, MarketDataDb, MetadataDb},
};

/// 数据库检查命令。
#[derive(Debug, clap::Args)]
pub struct TestCommand {}

impl TestCommand {
    pub(super) fn execute(self) {
        let config = Config::load_or_gen_default();
        if let Err(error) = self.execute_with(&config, ".") {
            eprintln!("导出数据库测试数据失败: {error}");
            std::process::exit(0);
        }
    }

    fn execute_with(&self, config: &Config, output: impl AsRef<Path>) -> io::Result<()> {
        let output = output.as_ref();
        fs::create_dir_all(output)?;

        let metadata_codes = export_metadata(&config.data.metadata, &output.join("metadata.txt"))?;
        let finance_codes = export_finance(&config.data.finance, &output.join("finance.txt"))?;
        let market_codes = export_market(&config.data.market, &output.join("market.txt"))?;

        report_code_differences(&metadata_codes, &finance_codes, &market_codes);
        println!("数据库测试数据已输出到: {}", output.display());
        Ok(())
    }
}

#[derive(Serialize)]
struct StockRecord<T> {
    code: String,
    data: Option<T>,
}

fn export_metadata(database_path: &Path, output_path: &Path) -> io::Result<BTreeSet<String>> {
    let database = MetadataDb::open_read_only(database_path).map_err(io::Error::other)?;
    let metadata = database.query_all().map_err(io::Error::other)?;
    let codes = metadata.iter().map(|item| item.code.to_string()).collect();
    let mut output = create_output(output_path)?;
    for item in metadata {
        write_json_line(&mut output, &item)?;
    }
    output.flush()?;
    Ok(codes)
}

fn export_finance(database_dir: &Path, output_path: &Path) -> io::Result<BTreeSet<String>> {
    let mut codes = BTreeSet::new();
    let mut output = create_output(output_path)?;
    for database_path in stock_databases(database_dir)? {
        let code = stock_code(&database_path)?;
        let database = FinanceDB::open_read_only(&database_path).map_err(io::Error::other)?;
        let data = database.query_latest().map_err(io::Error::other)?;
        codes.insert(code.clone());
        write_json_line(&mut output, &StockRecord { code, data })?;
    }
    output.flush()?;
    Ok(codes)
}

fn export_market(database_dir: &Path, output_path: &Path) -> io::Result<BTreeSet<String>> {
    let mut codes = BTreeSet::new();
    let mut output = create_output(output_path)?;
    for database_path in stock_databases(database_dir)? {
        let code = stock_code(&database_path)?;
        let database = MarketDataDb::open_read_only(&database_path).map_err(io::Error::other)?;
        let data = database.query_latest().map_err(io::Error::other)?;
        codes.insert(code.clone());
        write_json_line(&mut output, &StockRecord { code, data })?;
    }
    output.flush()?;
    Ok(codes)
}

fn report_code_differences(metadata_codes: &BTreeSet<String>, finance_codes: &BTreeSet<String>, market_codes: &BTreeSet<String>) {
    let mut all_codes = metadata_codes.clone();
    all_codes.extend(finance_codes.iter().cloned());
    all_codes.extend(market_codes.iter().cloned());

    let metadata_missing = print_missing_codes("metadata", &all_codes, metadata_codes);
    let finance_missing = print_missing_codes("finance", &all_codes, finance_codes);
    let market_missing = print_missing_codes("market", &all_codes, market_codes);

    if !metadata_missing && !finance_missing && !market_missing {
        println!("三份数据库股票代码一致，共 {} 个", all_codes.len());
    }
}

fn print_missing_codes(name: &str, all_codes: &BTreeSet<String>, codes: &BTreeSet<String>) -> bool {
    let missing = missing_codes(all_codes, codes);
    if missing.is_empty() {
        return false;
    }

    println!("{name} 缺少以下 {} 个股票代码:", missing.len());
    for code in missing {
        println!("{code}");
    }
    true
}

fn missing_codes(all_codes: &BTreeSet<String>, codes: &BTreeSet<String>) -> Vec<String> {
    all_codes.difference(codes).cloned().collect()
}

fn stock_databases(directory: &Path) -> io::Result<Vec<PathBuf>> {
    let mut databases = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<Vec<_>>>()?;
    databases.retain(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "db"));
    databases.sort();
    Ok(databases)
}

fn stock_code(database_path: &Path) -> io::Result<String> {
    database_path
        .file_stem()
        .and_then(|code| code.to_str())
        .map(str::to_owned)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("数据库文件名不是有效的股票代码: {}", database_path.display()),
            )
        })
}

fn create_output(path: &Path) -> io::Result<BufWriter<File>> {
    File::create(path).map(BufWriter::new)
}

fn write_json_line(output: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *output, value).map_err(io::Error::other)?;
    output.write_all(b"\n")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;
    use time::{Date, Month};

    use crate::{
        app::ParseCommand,
        config::{DataConfig, ServerConfig},
        db::{FinanceDB, MarketDataDb, MetadataDb},
    };

    use super::*;

    // 测试三份数据库代码不一致时，可以分别找出每份数据缺少的股票代码。
    #[test]
    fn compares_database_codes() {
        let metadata_codes = BTreeSet::from(["000001".to_string(), "000002".to_string()]);
        let finance_codes = BTreeSet::from(["000001".to_string(), "000003".to_string()]);
        let market_codes = BTreeSet::from(["000001".to_string(), "000002".to_string(), "000003".to_string()]);
        let mut all_codes = metadata_codes.clone();
        all_codes.extend(finance_codes.iter().cloned());
        all_codes.extend(market_codes.iter().cloned());

        assert_eq!(missing_codes(&all_codes, &metadata_codes), ["000003"]);
        assert_eq!(missing_codes(&all_codes, &finance_codes), ["000002"]);
        assert!(missing_codes(&all_codes, &market_codes).is_empty());
    }

    fn date() -> Date {
        Date::from_calendar_date(2025, Month::January, 1).unwrap()
    }

    // 测试 parse 和 test 命令按配置生成数据库，并导出最新数据。
    #[test]
    fn exports_configured_databases() {
        let directory = tempdir().unwrap();
        let tbf_market = directory.path().join("tbf/market");
        let tbf_finance = directory.path().join("tbf/finance");
        let tbf_metadata = directory.path().join("tbf/metadata");
        fs::create_dir_all(&tbf_market).unwrap();
        fs::create_dir_all(&tbf_finance).unwrap();
        fs::create_dir_all(&tbf_metadata).unwrap();

        fs::write(
            tbf_market.join("000001.tbf"),
            r#"<begin>{"datetime":"2025-01-01","change_percent":0.01,"open":10.0,"close":11.0,"high":12.0,"low":9.0,"volume":100.0,"turnover":1000.0,"turnover_rate":0.02,"is_st":false}</end><begin>{"datetime":"2025-01-02","change_percent":0.02,"open":11.0,"close":13.0,"high":14.0,"low":10.0,"volume":200.0,"turnover":2000.0,"turnover_rate":0.03,"is_st":false}</end>"#,
        )
        .unwrap();
        fs::write(
            tbf_finance.join("000001.tbf"),
            r#"<begin>{"datetime":"2025-01-01","total_shares":100.0,"float_shares":50.0,"total_market":1000.0,"float_market":500.0}</end><begin>{"datetime":"2025-01-02","total_shares":200.0,"float_shares":80.0,"total_market":2000.0,"float_market":800.0}</end>"#,
        )
        .unwrap();
        fs::write(
            tbf_metadata.join("000001.tbf"),
            r#"<begin>{"exchange":"SSE","name":"测试股票","code":"000001","prov":"上海","city":"上海","SW1":"行业一","SW2":"行业二","SW3":"行业三","indice":[],"listing_date":"2020-01-01"}</end>"#,
        )
        .unwrap();

        let config = Config {
            server: ServerConfig::default(),
            period: Vec::new(),
            data: DataConfig {
                cache: PathBuf::from("test_cache"),
                market: directory.path().join("database/market"),
                finance: directory.path().join("database/finance"),
                metadata: directory.path().join("database/metadata.db"),
                tbf_market,
                tbf_finance,
                tbf_metadata,
            },
        };

        ParseCommand {}.execute_with(&config).unwrap();

        let market = MarketDataDb::new(config.data.market.join("000001.db"))
            .unwrap()
            .query(date(), date())
            .unwrap();
        let finance = FinanceDB::new(config.data.finance.join("000001.db"))
            .unwrap()
            .query(date(), date())
            .unwrap();
        let metadata = MetadataDb::new(&config.data.metadata).unwrap().query("000001").unwrap().unwrap();

        assert_eq!(market.len(), 1);
        assert_eq!(market[0].close, 11.0);
        assert_eq!(finance.len(), 1);
        assert_eq!(finance[0].total_shares, 100.0);
        assert_eq!(metadata.name.as_ref(), "测试股票");

        TestCommand {}.execute_with(&config, directory.path()).unwrap();

        let metadata_output = fs::read_to_string(directory.path().join("metadata.txt")).unwrap();
        let finance_output = fs::read_to_string(directory.path().join("finance.txt")).unwrap();
        let market_output = fs::read_to_string(directory.path().join("market.txt")).unwrap();
        let metadata_json: serde_json::Value = serde_json::from_str(metadata_output.trim()).unwrap();
        let finance_json: serde_json::Value = serde_json::from_str(finance_output.trim()).unwrap();
        let market_json: serde_json::Value = serde_json::from_str(market_output.trim()).unwrap();

        assert_eq!(metadata_output.lines().count(), 1);
        assert_eq!(metadata_json["code"], "000001");
        assert_eq!(finance_output.lines().count(), 1);
        assert_eq!(finance_json["code"], "000001");
        assert_eq!(finance_json["data"]["datetime"], "2025-01-02");
        assert_eq!(market_output.lines().count(), 1);
        assert_eq!(market_json["code"], "000001");
        assert_eq!(market_json["data"]["datetime"], "2025-01-02");
        assert_eq!(market_json["data"]["close"], 13.0);
    }
}
