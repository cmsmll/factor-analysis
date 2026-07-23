//! 价量趋势因子。

use std::sync::Arc;

use salvo::{Router, Writer};
use salvo_oapi::{ToSchema, endpoint};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Receiver;

use crate::{math::dev, prelude::*, reject, resolve, resp::Resp, router::mode1::Base, toolbox::VJson};

/// 价量趋势因子分析请求。
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
        let recv = MODE1.cache.get_or_run(key, move || pvt_run(req));
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

/// 注册价量趋势因子。
pub async fn router() -> Router {
    MODE1.register(Arc::new(Req::register)).await;
    Router::with_path(Req::id()).post(pvt)
}

/// 按当日价量趋势值进行分位分析。
#[endpoint(
    tags("模式一"),
    operation_id = "analyze_pvt",
    responses(
        (status_code = 200, description = "价量趋势因子分析结果", body = Res<QuantileData>),
        (status_code = 400, description = "分析任务失败", body = Res<()>),
        (status_code = 422, description = "参数校验失败", body = Res<()>),
        (status_code = 415, description = "Content-Type 或 JSON 请求体错误", body = Res<()>),
    )
)]
pub async fn pvt(args: VJson<Req>) -> Resp<Arc<RawValue>> {
    let key = args.0.hashcode();
    match MODE1.cache.get_or_run(key, move || pvt_run(args.0)).recv().await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

fn pvt_run(args: Req) -> Box<RawValue> {
    let df = DF.filter(&args.base.filter);
    let mut qd = QuantileData::new("价量趋势因子(PVT)", "PVT:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*VOLUME", args.base.count);
    let mut items = Vec::with_capacity(df.list.len());

    for index in df.index_iter() {
        for item in &df.list {
            if let Some(curr) = item.data_ref(&index)
                && curr.filter_st(args.base.filter_st)
                && let Some(prev) = curr.before(1)
                && let Some(profit) = item.profit.get(curr.index())
            {
                items.push(TempItem {
                    factor: pvt_factor(curr.close, prev.close, curr.volume),
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
fn pvt_factor(close: f64, prev_close: f64, volume: f64) -> f64 {
    dev(close - prev_close, prev_close) * volume
}
