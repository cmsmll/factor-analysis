//! 总市值因子接口。

use std::sync::Arc;

use salvo::{Router, Writer};
use salvo_oapi::{ToSchema, endpoint};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Receiver;

use crate::{prelude::*, reject, resolve, resp::Resp, router::mode1::Base, toolbox::VJson};

/// 总市值因子分析请求。
///
/// 客户端通常先从 `POST /api/mode1/list` 取得默认结构，再按需修改参数。
#[derive(Debug, Serialize, Deserialize, ToSchema, validator::Validate)]
pub struct Req {
    #[validate(nested)]
    base: Base,
}

impl Req {
    fn register(filter: &Filter) -> (Arc<RawValue>, Receiver<Arc<RawValue>>) {
        let mut req = Self::default();
        req.base.filter = filter.clone();
        let value = Arc::from(req.raw_value());
        let key = req.hashcode();
        let recv = MODE1.cache.get_or_run(key, move || market_value_run(req));
        (value, recv)
    }
}

impl ArgsHandle for Req {}
impl Default for Req {
    fn default() -> Self {
        Self {
            base: Base {
                id: Self::id(),
                count: 5,
                filter: Filter::from_config(&CONFIG),
            },
        }
    }
}

/// 注册总市值因子接口。
///
/// 动态 `factor_id` 应通过 `POST /api/mode1/list` 获取。
pub async fn router() -> Router {
    MODE1.register(Arc::new(Req::register)).await;
    Router::with_path(Req::id()).post(market_value)
}

/// 执行总市值因子的分位分析。
///
/// # Analysis
///
/// 每个交易日直接读取对齐财务数据中的 `total_market` 作为总市值，
/// 按总市值从低到高切分为 `base.count` 个分位，因子值单位为元。
#[endpoint(
    tags("模式一"),
    operation_id = "analyze_market_value",
    responses(
        (status_code = 200, description = "总市值因子分析结果", body = Res<QuantileData>),
        (status_code = 400, description = "分析任务失败", body = Res<()>),
        (status_code = 422, description = "参数校验失败", body = Res<()>),
        (status_code = 415, description = "Content-Type 或 JSON 请求体错误", body = Res<()>),
    )
)]
pub async fn market_value(args: VJson<Req>) -> Resp<Arc<RawValue>> {
    let key = args.0.hashcode();
    match MODE1.cache.get_or_run(key, move || market_value_run(args.0)).recv().await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

/// 根据总市值计算每日分位数据和四种收益。
fn market_value_run(args: Req) -> Box<RawValue> {
    let df = DF.filter(&args.base.filter);
    let mut qd = QuantileData::new("总市值因子", "按总市值从低到高分位", args.base.count);
    let mut items = Vec::with_capacity(df.list.len());

    for index in df.index_iter() {
        for item in &df.list {
            if let Some((curr, profit, finance)) = item.data_and_finance(&index)
                && curr.filter_st(args.base.filter_st)
            {
                items.push(TempItem {
                    factor: finance.total_market,
                    profit,
                });
            }
        }
        qd.push(index.datetime, &mut items);
        // items.clear();
        unsafe { items.set_len(0) }
    }

    qd.raw_value()
}
