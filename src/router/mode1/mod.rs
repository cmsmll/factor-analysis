//! 因子分位分析

use salvo::Router;
pub mod turnover_rate;

pub async fn mode1_router() -> Router {
    Router::with_path("mode1").push(turnover_rate::router().await)
}
