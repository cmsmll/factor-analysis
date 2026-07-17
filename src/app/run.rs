use salvo::{cors::Cors, prelude::*};

use crate::{CONFIG, logger::Logger, router};

/// Web 服务运行命令。
#[derive(Debug, clap::Args)]
pub struct RunCommand {}

impl RunCommand {
    pub(super) async fn execute(self) {
        let router = router::router().await;
        let addr = CONFIG.socket_addr();

        println!("{:?}", router);
        println!("WebService running at: http://{addr}");

        let cors = Cors::permissive().into_handler();
        let acceptor = TcpListener::new(addr).bind().await;
        let server = Service::new(router).hoop(cors).hoop(Logger::default());
        Server::new(acceptor).serve(server).await;
    }
}
