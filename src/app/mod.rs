mod parse;
mod run;
mod test;

use clap::Parser;

pub use parse::ParseCommand;
pub use run::RunCommand;
pub use test::TestCommand;

/// 因子分析命令行入口。
#[derive(Debug, Parser)]
#[command(name = "factor-analysis", version, about = "因子分析服务")]
pub enum App {
    /// 解析 Tbf 数据。
    Parse(ParseCommand),
    /// 运行 Web 服务。
    Run(RunCommand),
    /// 检查数据库并导出文本数据。
    Test(TestCommand),
}

impl App {
    /// 执行解析后的子命令。
    pub async fn execute(self) {
        match self {
            Self::Parse(command) => command.execute(),
            Self::Run(command) => command.execute().await,
            Self::Test(command) => command.execute(),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    // 测试 parse 子命令可以通过 Clap 派生参数完成解析。
    #[test]
    fn parses_parse_command() {
        let app = App::try_parse_from(["factor-analysis", "parse"]).unwrap();

        assert!(matches!(app, App::Parse(_)));
    }

    // 测试 run 子命令可以通过 Clap 派生参数完成解析。
    #[test]
    fn parses_run_command() {
        let app = App::try_parse_from(["factor-analysis", "run"]).unwrap();

        assert!(matches!(app, App::Run(_)));
    }

    // 测试 test 子命令可以通过 Clap 派生参数完成解析。
    #[test]
    fn parses_test_command() {
        let app = App::try_parse_from(["factor-analysis", "test"]).unwrap();

        assert!(matches!(app, App::Test(_)));
    }
}
