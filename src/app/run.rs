use salvo::prelude::*;

use crate::{CONFIG, logger::Logger, resolve, resp::Resp, router::api_router};

/// Web 服务运行命令。
#[derive(Debug, clap::Args)]
pub struct RunCommand {}

impl RunCommand {
    pub(super) async fn execute(self) {
        let router = Router::new().hoop(Logger::default()).push(api_router()).get(hello);
        let addr = CONFIG.socket_addr();

        println!("WebService running at: http://{addr}");
        let acceptor = TcpListener::new(addr).bind().await;
        Server::new(acceptor).serve(router).await;
    }
}

#[handler]
async fn hello() -> Resp<&'static str> {
    resolve!("Hello World" => 200, "ok")
}
