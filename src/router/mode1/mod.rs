//! 模式一：按照因子值排序并进行分位分析。

use derive_more::Deref;
use salvo_oapi::{ToSchema, endpoint};
use serde::{Deserialize, Serialize};

use crate::{MODE1, prelude::*, toolbox::VJson};

pub mod amplitude;
pub mod bbi;
pub mod bias_n;
pub mod cci_n;
pub mod ema_close_n;
pub mod macd;
pub mod manager;
pub mod market_value;
pub mod mass;
pub mod pvt;
pub mod pvt_n;
pub mod sma_close_n;
pub mod trix_n;
pub mod turnover;
pub mod turnover_n;
pub mod turnover_rate;
pub mod turnover_rate_n;
pub mod volume;
pub mod volume_n;

/// 模式一因子的公共请求参数。
#[derive(Debug, Serialize, Deserialize, ToSchema, Deref, validator::Validate)]
pub struct Base {
    /// 动态接口 ID，应使用 `/api/mode1/list` 返回的值。
    pub id: String,
    /// 分位数量，调用方应保证大于等于 1。
    #[validate(range(min = 1, message = "分位数量必须大于等于 1"))]
    pub count: usize,
    /// 股票池与日期筛选条件。
    #[deref]
    #[validate(nested)]
    pub filter: Filter,
}

fn validate_period(period: &UntArg) -> Result<(), validator::ValidationError> {
    if period.value >= 2 {
        Ok(())
    } else {
        Err(validator::ValidationError::new("period_min").with_message("周期必须大于等于 2".into()))
    }
}

/// OpenAPI 中用于描述模式一模板列表的数据结构。
#[derive(Debug, ToSchema)]
pub struct Mode1Template {
    /// 因子的公共请求参数。
    pub base: Base,
}

/// 构建模式一的路由树。
pub async fn mode1_router() -> Router {
    Router::with_path("mode1")
        .push(Router::with_path("list").post(list))
        .push(turnover_rate::router().await)
        .push(amplitude::router().await)
        .push(market_value::router().await)
        .push(volume::router().await)
        .push(turnover::router().await)
        .push(volume_n::router().await)
        .push(turnover_n::router().await)
        .push(turnover_rate_n::router().await)
        .push(bias_n::router().await)
        .push(cci_n::router().await)
        .push(sma_close_n::router().await)
        .push(ema_close_n::router().await)
        .push(pvt::router().await)
        .push(pvt_n::router().await)
        .push(macd::router().await)
        .push(bbi::router().await)
        .push(mass::router().await)
        .push(trix_n::router().await)
}

/// 按筛选条件获取模式一因子的请求参数和分析结果。
///
/// 请求体为股票池和日期筛选条件。接口并发执行所有已注册的模式一任务，
/// 返回每个因子的实际请求参数和对应分析结果。
#[endpoint(
    tags("模式一"),
    operation_id = "list_mode1_factors",
    responses(
        (status_code = 200, description = "模式一因子参数和分析结果列表"),
        (status_code = 422, description = "参数校验失败", body = Res<()>)
    )
)]
pub(super) async fn list(filter: VJson<Filter>) -> Res<Vec<manager::ListItem>> {
    res!(MODE1.execute(&filter.0).await => 200, "ok")
}
