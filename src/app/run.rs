use std::sync::Arc;

use salvo::prelude::*;

use crate::{config::Config, logger::Logger, resolve, resp::Resp};

/// Web 服务运行命令。
#[derive(Debug, clap::Args)]
pub struct RunCommand {}

impl RunCommand {
    pub(super) async fn execute(self) {
        let config = Config::load_or_gen_default();
        let router = Router::new()
            .hoop(Logger::default())
            .get(hello)
            .push(Router::with_path("number").get(number));
        let addr = config.socket_addr();
        println!("WebService running at: http://{addr}");
        let acceptor = TcpListener::new(addr).bind().await;
        Server::new(acceptor).serve(router).await;
    }
}

#[handler]
async fn hello(depot: &mut Depot) -> Resp<&'static str> {
    depot.insert("error", Arc::from("测试error") as Arc<str>);
    depot.insert("other", Arc::from("测试other") as Arc<str>);

    resolve!("Hello World" => 200, "ok")
}

#[handler]
async fn number(depot: &mut Depot) -> Resp<()> {
    depot.insert("error", Arc::from("测试error") as Arc<str>);
    depot.insert("other", Arc::from("测试other") as Arc<str>);

    let _: u32 = "a123".parse()?;
    resolve!(200, "ok")
}
