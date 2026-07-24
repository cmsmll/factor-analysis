//! 价量趋势因子N日平均。

use std::sync::Arc;

use salvo::{Router, Writer};
use salvo_oapi::{ToSchema, endpoint};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Receiver;

use crate::{
    math::{SMA, dev},
    prelude::*,
    reject, resolve,
    resp::Resp,
    router::mode1::Base,
    toolbox::VJson,
};

#[derive(Debug, Serialize, Deserialize, ToSchema, validator::Validate)]
pub struct Core {
    /// 计算周期，单位为交易日。
    #[validate(custom(function = "super::validate_period"))]
    pub period: UntArg,
}

impl Core {
    pub fn new(period: usize) -> Self {
        assert!(period >= 2, "周期必须大于等于 2");
        Self {
            period: UntArg::new("多日周期", period),
        }
    }
}

/// 多日平均价量趋势因子分析请求。
#[derive(Debug, Serialize, Deserialize, ToSchema, validator::Validate)]
pub struct Req {
    #[validate(nested)]
    base: Base,
    #[validate(nested)]
    core: Core,
}

impl Req {
    fn new(period: usize) -> Self {
        Self {
            base: Base {
                id: Self::id(),
                count: 5,
                filter: Filter::from_config(&CONFIG),
            },
            core: Core::new(period),
        }
    }

    fn register(filter: &Filter, period: usize) -> (Arc<RawValue>, Receiver<Arc<RawValue>>) {
        let mut req = Self::new(period);
        req.base.filter = filter.clone();
        let value = Arc::from(req.raw_value());
        let key = req.hashcode();
        let recv = MODE1.cache.get_or_run(key, move || pvt_n_run(req));
        (value, recv)
    }
}

impl ArgsHandle for Req {}

/// 注册 6 日和 10 日平均价量趋势因子。
pub async fn router() -> Router {
    MODE1.register(Arc::new(|filter| Req::register(filter, 6))).await;
    MODE1.register(Arc::new(|filter| Req::register(filter, 10))).await;
    Router::new().push(Router::with_path(Req::id()).post(pvt_n))
}

/// 按 N 日平均价量趋势值进行分位分析。
#[endpoint(
    tags("模式一"),
    operation_id = "analyze_pvt_n",
    responses(
        (status_code = 200, description = "多日平均价量趋势因子分析结果", body = Res<Mode1Data>),
        (status_code = 400, description = "分析任务失败", body = Res<()>),
        (status_code = 422, description = "参数校验失败", body = Res<()>),
        (status_code = 415, description = "Content-Type 或 JSON 请求体错误", body = Res<()>),
    )
)]
pub async fn pvt_n(args: VJson<Req>) -> Resp<Arc<RawValue>> {
    let key = args.0.hashcode();
    match MODE1.cache.get_or_run(key, move || pvt_n_run(args.0)).recv().await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

fn pvt_n_run(args: Req) -> Box<RawValue> {
    let period = args.core.period.value;
    let df = DF.filter(&args.base.filter);
    let mut qd = Mode1Data::new(
        format!("价量趋势因子(PVT){period}日平均"),
        format!("PVT_N:=MA(PVT,N); PVT:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*VOLUME; N:={period}"),
        super::LABEL,
        args.base.count,
    );
    let mut items = Vec::with_capacity(df.list.len());
    let mut store = vec![SMA::new(period); df.list.len()];

    for index in df.index_iter() {
        for (item, store) in df.list.iter().zip(store.iter_mut()) {
            if let Some((curr, profit)) = item.data(&index)
                && curr.filter_st(args.base.filter_st)
                && let Some(prev1) = item.before(&index, 1)
                && let Some(factor) = store.next(pvt_factor(curr.close, prev1.close, curr.volume))
            {
                items.push(Mode1Temp { factor, profit });
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
