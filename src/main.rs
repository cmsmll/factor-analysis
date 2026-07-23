use clap::Parser;
use factor_analysis::App;

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    App::parse().execute().await;
}
