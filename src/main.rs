use std::sync::Arc;

use bitcode::{Encode, encode};
use factor_analysis::{logger::Logger, resolve, resp::Resp};
use salvo::prelude::*;

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

#[tokio::main]
async fn main() {
    let router = Router::new()
        .hoop(Logger::default())
        .get(hello)
        .push(Router::with_path("number").get(number));
    let acceptor = TcpListener::new("127.0.0.1:7878").bind().await;
    Server::new(acceptor).serve(router).await;
}

#[derive(Default, Debug, Encode)]
pub struct Args {
    name: String,
    label: String,
    filter_st: bool,         // 过滤ST
    sector: Option<String>,  // 板块
    variety: Option<String>, // 品种
}

impl Args {
    pub fn hash(&self) -> String {
        let buf = encode(self);
        let res = blake3::hash(&buf);
        res.to_string()
    }
}
