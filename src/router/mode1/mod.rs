//! 模式一：按照因子值排序并进行分位分析。

use salvo_oapi::{ToSchema, endpoint};
use serde::{Deserialize, Serialize};

use crate::{MODE1, prelude::*};

pub mod amplitude;
pub mod turnover_rate;

/// 模式一因子的公共请求参数。
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Base {
    /// 动态接口 ID，应使用 `/api/mode1/list` 返回的值。
    pub id: String,
    /// 分位数量，调用方应保证大于等于 1。
    pub count: usize,
    /// 股票池与日期筛选条件。
    pub filter: Filter,
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
        .push(Router::with_path("list").get(list))
        .push(turnover_rate::router().await)
        .push(amplitude::router().await)
}

/// 获取模式一因子的默认请求模板。
///
/// 当前依次返回换手率因子和振幅因子模板。模板中的 `base.id` 是实际接口路径 ID。
#[endpoint(
    tags("模式一"),
    operation_id = "list_mode1_factors",
    responses((status_code = 200, description = "模式一因子模板列表", body = Res<Vec<Mode1Template>>))
)]
pub(super) fn list() -> Res<Box<RawValue>> {
    let value = MODE1.lock().unwrap();
    let value = serde_json::to_string(&*value).unwrap();
    let value = RawValue::from_string(value).unwrap();
    res!(value => 200, "ok")
}
