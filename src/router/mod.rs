//! HTTP API 路由。
//!
//! OpenAPI JSON 位于 `/api-doc/openapi.json`，Swagger UI 位于 `/swagger-ui`。

pub mod mode1;

use std::{collections::HashSet, sync::Arc};

use salvo::prelude::*;
use salvo_oapi::endpoint;
use serde_json::value::RawValue;
use time::macros::date;

use crate::{
    CONFIG, DF, MODE1,
    config::Period,
    model::{QuantileData, TempItem},
    reject, res, resolve,
    resp::{Res, Resp},
};

/// 构建应用的业务接口路由。
pub async fn router() -> Router {
    println!("股票池数量: {}", DF.list.len());
    println!("开始时间: {}", DF.start);
    println!("结束时间: {}", DF.end);

    Router::new()
        .push(
            Router::with_path("api")
                .push(mode1::mode1_router().await)
                .push(Router::with_path("indice").get(indice))
                .push(Router::with_path("sector").get(sector))
                .push(Router::with_path("period").get(period))
                .push(Router::with_path("test").get(test)),
        )
        .get(hello)
}

/// 获取股票池指数列表。
///
/// 返回所有合约元数据中 `indice` 字段的去重集合。集合序列化后的顺序不固定。
#[endpoint(
    tags("基础数据"),
    operation_id = "list_indices",
    responses((status_code = 200, description = "指数列表", body = Res<HashSet<String>>))
)]
fn indice() -> Res<Arc<HashSet<String>>> {
    res!(DF.indice.clone() => 200, "ok")
}

/// 获取股票池行业板块列表。
///
/// 返回所有合约元数据中 `SW1`、`SW2`、`SW3` 的非空去重集合。
#[endpoint(
    tags("基础数据"),
    operation_id = "list_sectors",
    responses((status_code = 200, description = "行业板块列表", body = Res<HashSet<String>>))
)]
fn sector() -> Res<Arc<HashSet<String>>> {
    res!(DF.sector.clone() => 200, "ok")
}

#[endpoint]
fn period() -> Res<Vec<Period>> {
    res!(CONFIG.period.clone() => 200, "ok")
}

/// 服务健康检查。
#[endpoint(
    tags("系统"),
    operation_id = "health_check",
    responses((status_code = 200, description = "服务正常", body = Res<String>))
)]
async fn hello() -> Resp<&'static str> {
    resolve!("Hello World" => 200, "ok")
}

/// 执行固定参数的测试换手率分析。
///
/// 使用 `2025-01-01` 至 `2025-12-31` 和 5 个分位，结果通过固定键 `test` 缓存。
#[endpoint(
    tags("测试"),
    operation_id = "run_test_analysis",
    responses(
        (status_code = 200, description = "测试分析结果", body = Res<QuantileData>),
        (status_code = 400, description = "分析任务失败", body = Res<()>),
    )
)]
async fn test() -> Resp<Arc<RawValue>> {
    match MODE1.cache.get_or_run(Arc::from("test"), test_run).recv().await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

/// 计算测试接口使用的固定换手率分位数据。
fn test_run() -> Box<RawValue> {
    let df = DF.range(date!(2025 - 01 - 01), date!(2025 - 12 - 31));
    let mut qd: QuantileData = QuantileData::new("测试换手率因子", "", 5);
    let mut items = Vec::with_capacity(df.list.len());

    for index in df.index_iter() {
        for item in &df.list {
            if let Some((curr, profit)) = item.data(&index) {
                items.push(TempItem {
                    factor: curr.turnover_rate,
                    profit,
                });
            }
        }
        qd.push(index.datetime, &mut items);
        items.clear();
    }

    qd.raw_value()
}
#[cfg(test)]
mod tests {
    use super::*;

    // 测试 OpenAPI 文档包含所有接口操作、JSON 请求体和主要响应模型。
    #[test]
    fn openapi_contains_documented_operations() {
        let router = Router::new().get(hello).push(
            Router::with_path("api")
                .push(Router::with_path("indice").get(indice))
                .push(Router::with_path("sector").get(sector))
                .push(Router::with_path("test").get(test))
                .push(
                    Router::with_path("mode1")
                        .push(Router::with_path("list").post(mode1::list))
                        .push(Router::with_path("turnover-rate").post(mode1::turnover_rate::turnover_rate))
                        .push(Router::with_path("amplitude").post(mode1::amplitude::amplitude))
                        .push(Router::with_path("market-value").post(mode1::market_value::market_value))
                        .push(Router::with_path("volume").post(mode1::volume::volume))
                        .push(Router::with_path("turnover").post(mode1::turnover::turnover))
                        .push(Router::with_path("volume-n").post(mode1::volume_n::volume_n))
                        .push(Router::with_path("turnover-n").post(mode1::turnover_n::turnover_n))
                        .push(Router::with_path("turnover-rate-n").post(mode1::turnover_rate_n::turnover_rate_n))
                        .push(Router::with_path("bias-n").post(mode1::bias_n::bias_n))
                        .push(Router::with_path("cci-n").post(mode1::cci_n::cci_n))
                        .push(Router::with_path("sma-close-n").post(mode1::sma_close_n::sma_close_n))
                        .push(Router::with_path("ema-close-n").post(mode1::ema_close_n::ema_close_n))
                        .push(Router::with_path("pvt").post(mode1::pvt::pvt))
                        .push(Router::with_path("pvt-n").post(mode1::pvt_n::pvt_n))
                        .push(Router::with_path("macd").post(mode1::macd::macd))
                        .push(Router::with_path("bbi").post(mode1::bbi::bbi))
                        .push(Router::with_path("mass").post(mode1::mass::mass))
                        .push(Router::with_path("trix-n").post(mode1::trix_n::trix_n)),
                ),
        );
        let document = crate::app::build_openapi(&router);
        let json = document.to_json().unwrap();

        for operation in [
            "health_check",
            "list_indices",
            "list_sectors",
            "run_test_analysis",
            "list_mode1_factors",
            "analyze_turnover_rate",
            "analyze_amplitude",
            "analyze_market_value",
            "analyze_volume",
            "analyze_turnover",
            "analyze_volume_n",
            "analyze_turnover_n",
            "analyze_turnover_rate_n",
            "analyze_bias_n",
            "analyze_cci_n",
            "analyze_sma_close_n",
            "analyze_ema_close_n",
            "analyze_pvt",
            "analyze_pvt_n",
            "analyze_macd",
            "analyze_bbi",
            "analyze_mass",
            "analyze_trix_n",
        ] {
            assert!(json.contains(operation), "OpenAPI 缺少操作: {operation}");
        }
        assert!(json.contains("requestBody"));
        assert!(json.contains("QuantileData"));
        assert!(json.contains("415"));
        let document: serde_json::Value = serde_json::from_str(&json).unwrap();
        let paths = document["paths"].as_object().unwrap();
        for path in paths.values() {
            for operation in path.as_object().unwrap().values() {
                if let Some(responses) = operation.get("responses").and_then(|value| value.as_object()) {
                    assert!(!responses.contains_key("default"));
                }
            }
        }
    }
}
