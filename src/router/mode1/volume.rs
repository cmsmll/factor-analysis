//! 成交量因子接口。

use std::sync::Arc;

use salvo::{Router, Writer};
use salvo_oapi::{ToSchema, endpoint};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Receiver;

use crate::{db::MarketData, prelude::*, reject, resolve, resp::Resp, router::mode1::Base, toolbox::VJson};

/// 成交量因子分析请求。
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
        let recv = MODE1.cache.get_or_run(key, move || volume_run(req));
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

/// 注册成交量因子接口，并加入模式一因子列表。
pub async fn router() -> Router {
    MODE1.register(Arc::new(Req::register)).await;
    Router::with_path(Req::id()).post(volume)
}

/// 执行成交量因子的分位分析。
///
/// 每个交易日按当日成交量从低到高排序，切分为 `base.count` 个分位，
/// 并返回各分位的平均成交量、平均换手率和四种平均收益。
#[endpoint(
    tags("模式一"),
    operation_id = "analyze_volume",
    responses(
        (status_code = 200, description = "成交量因子分析结果", body = Res<QuantileData>),
        (status_code = 400, description = "分析任务失败", body = Res<()>),
        (status_code = 422, description = "参数校验失败", body = Res<()>),
        (status_code = 415, description = "Content-Type 或 JSON 请求体错误", body = Res<()>),
    )
)]
pub async fn volume(args: VJson<Req>) -> Resp<Arc<RawValue>> {
    let key = args.0.hashcode();
    match MODE1.cache.get_or_run(key, move || volume_run(args.0)).recv().await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

fn volume_run(args: Req) -> Box<RawValue> {
    let df = DF.filter(&args.base.filter);
    let mut qd = QuantileData::new("成交量因子", "按成交量从低到高分位", args.base.count);
    let mut items = Vec::with_capacity(df.list.len());

    for index in df.index_iter() {
        for item in &df.list {
            if let Some((curr, profit)) = item.data(&index)
                && curr.filter_st(args.base.filter_st)
            {
                items.push(TempItem {
                    factor: volume_factor(curr),
                    profit,
                });
            }
        }
        qd.push(index.datetime, &mut items);
        items.clear();
    }

    qd.raw_value()
}

#[inline]
fn volume_factor(market: &MarketData) -> f64 {
    market.volume
}
