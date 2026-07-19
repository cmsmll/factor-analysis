//! 模式一：按照因子值排序并进行分位分析。

use derive_more::Deref;
use salvo_oapi::{ToSchema, endpoint};
use serde::{Deserialize, Serialize};

use crate::{MODE1, prelude::*, toolbox::Json};

pub mod amplitude;
pub mod manager;
pub mod market_value;
pub mod turnover;
pub mod turnover_rate;
pub mod volume;

/// 模式一因子的公共请求参数。
#[derive(Debug, Serialize, Deserialize, ToSchema, Deref)]
pub struct Base {
    /// 动态接口 ID，应使用 `/api/mode1/list` 返回的值。
    pub id: String,
    /// 分位数量，调用方应保证大于等于 1。
    pub count: usize,
    /// 股票池与日期筛选条件。
    #[deref]
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
        .push(Router::with_path("list").post(list))
        .push(turnover_rate::router().await)
        .push(amplitude::router().await)
        .push(market_value::router().await)
        .push(volume::router().await)
        .push(turnover::router().await)
}

/// 按筛选条件获取模式一因子的请求参数和分析结果。
///
/// 请求体为股票池和日期筛选条件。接口并发执行所有已注册的模式一任务，
/// 返回每个因子的实际请求参数和对应分析结果。
#[endpoint(
    tags("模式一"),
    operation_id = "list_mode1_factors",
    responses((status_code = 200, description = "模式一因子参数和分析结果列表"))
)]
pub(super) async fn list(filter: Json<Filter>) -> Res<Vec<manager::ListItem>> {
    res!(MODE1.execute(&filter.0).await => 200, "ok")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use salvo::{
        Service,
        http::StatusCode,
        prelude::Router,
        test::{ResponseExt, TestClient},
    };
    use serde_json::{Value, json, value::RawValue};
    use time::macros::date;
    use tokio::sync::broadcast;

    use super::list;
    use crate::{MODE1, args::Filter};

    // 测试 POST /api/mode1/list 接收 Filter，并返回 MODE1 执行后的参数与数据。
    #[tokio::test]
    async fn list_executes_registered_tasks_with_request_filter() {
        MODE1
            .register(Arc::new(|filter| {
                let args = json!({
                    "base": {
                        "id": "interface-test",
                        "count": 5,
                        "filter": filter,
                    }
                });
                let args = Arc::from(RawValue::from_string(args.to_string()).unwrap());
                let data = Arc::from(RawValue::from_string(r#"{"name":"接口测试"}"#.to_owned()).unwrap());
                let (sender, receiver) = broadcast::channel(1);
                sender.send(data).unwrap();
                (args, receiver)
            }))
            .await;

        let mut filter = Filter::new(date!(2024 - 01 - 02), date!(2025 - 06 - 30));
        filter.filter_bz = true;
        let router = Router::with_path("api").push(Router::with_path("mode1").push(Router::with_path("list").post(list)));
        let service = Service::new(router);

        let mut response = TestClient::post("http://localhost/api/mode1/list").json(&filter).send(&service).await;

        assert_eq!(response.status_code, Some(StatusCode::OK));
        let body: Value = response.take_json().await.unwrap();
        assert_eq!(body["code"], 200);
        assert_eq!(body["info"], "ok");

        let item = body["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["args"]["base"]["id"] == "interface-test")
            .expect("响应中应包含接口测试任务");
        assert_eq!(item["args"]["base"]["filter"]["start"], "2024-01-02");
        assert_eq!(item["args"]["base"]["filter"]["end"], "2025-06-30");
        assert_eq!(item["args"]["base"]["filter"]["filter_bz"], true);
        assert_eq!(item["data"]["name"], "接口测试");
    }
}
