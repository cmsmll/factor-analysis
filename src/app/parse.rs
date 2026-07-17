use std::{io, path::Path, time::Instant};

use crate::{
    config::Config,
    db::{tbf_to_finance, tbf_to_market, tbf_to_metadata},
};

/// 原始数据解析命令。
#[derive(Debug, clap::Args)]
pub struct ParseCommand {}

impl ParseCommand {
    pub(super) fn execute(self) {
        let config = Config::load_or_gen_default();
        let total_start = Instant::now();
        if let Err(error) = self.execute_with(&config) {
            eprintln!("解析 TBF 数据失败: {error}");
            eprintln!("任务终止，总耗时: {:.2?}", total_start.elapsed());
            std::process::exit(0);
        }
        println!("全部任务完成，总耗时: {:.2?}", total_start.elapsed());
    }

    pub(super) fn execute_with(&self, config: &Config) -> io::Result<()> {
        let data = &config.data;
        execute_parse_task(
            "行情数据",
            &data.tbf_market,
            "行情 TBF 目录",
            &data.market,
            "行情数据库目录",
            tbf_to_market,
        )?;
        execute_parse_task(
            "财务数据",
            &data.tbf_finance,
            "财务 TBF 目录",
            &data.finance,
            "财务数据库目录",
            tbf_to_finance,
        )?;
        execute_parse_task(
            "元数据",
            &data.tbf_metadata,
            "元数据 TBF 目录",
            &data.metadata,
            "元数据数据库文件",
            tbf_to_metadata,
        )?;

        Ok(())
    }
}

fn execute_parse_task(
    name: &str,
    input: &Path,
    input_name: &str,
    output: &Path,
    output_name: &str,
    task: fn(&str, &str) -> io::Result<()>,
) -> io::Result<()> {
    let start = Instant::now();
    task(path_as_str(input, input_name)?, path_as_str(output, output_name)?).map_err(|error| io::Error::other(format!("{name}任务失败: {error}")))?;
    println!("{name}任务完成，耗时: {:.2?}", start.elapsed());

    Ok(())
}

fn path_as_str<'a>(path: &'a Path, name: &str) -> io::Result<&'a str> {
    path.to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("{name}不是有效的 UTF-8 路径: {}", path.display())))
}
