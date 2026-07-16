use clap::Parser;
use factor_analysis::App;

#[tokio::main]
async fn main() {
    App::parse().execute().await;
}
