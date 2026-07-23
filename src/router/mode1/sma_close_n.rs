//! 收盘价移动均线因子。

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

/// 收盘价移动均线因子分析请求。
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
        let recv = CACHE.get_or_run(key, move || sma_close_n_run(req));
        (value, recv)
    }
}

impl ArgsHandle for Req {}

/// 注册 10 日和 20 日收盘价移动均线因子。
pub async fn router() -> Router {
    MODE1.register(Arc::new(|filter| Req::register(filter, 10))).await;
    MODE1.register(Arc::new(|filter| Req::register(filter, 20))).await;
    Router::new().push(Router::with_path(Req::id()).post(sma_close_n))
}

/// 按 N 日收盘价简单移动均线与今日收盘价的比值进行分位分析。
#[endpoint(
    tags("模式一"),
    operation_id = "analyze_sma_close_n",
    responses(
        (status_code = 200, description = "收盘价移动均线因子分析结果", body = Res<QuantileData>),
        (status_code = 400, description = "分析任务失败", body = Res<()>),
        (status_code = 422, description = "参数校验失败", body = Res<()>),
        (status_code = 415, description = "Content-Type 或 JSON 请求体错误", body = Res<()>),
    )
)]
pub async fn sma_close_n(args: VJson<Req>) -> Resp<Arc<RawValue>> {
    let key = args.0.hashcode();
    match CACHE.get_or_run(key, move || sma_close_n_run(args.0)).recv().await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

fn sma_close_n_run(args: Req) -> Box<RawValue> {
    let period = args.core.period.value;
    let df = DF.filter(&args.base.filter);
    let mut qd = QuantileData::new(
        format!("移动均线因子(SMA){period}日"),
        format!("SMA_CLOSE_N:=MA(CLOSE,N)/CLOSE; N:={period}"),
        args.base.count,
    );
    let mut items = Vec::with_capacity(df.list.len());
    let mut store = vec![SMA::new(period); df.list.len()];

    for index in df.index_iter() {
        for (item, store) in df.list.iter().zip(store.iter_mut()) {
            if let Some((curr, profit)) = item.data(&index)
                && curr.filter_st(args.base.filter_st)
                && let Some(avg_close) = store.next(curr.close)
            {
                items.push(TempItem {
                    factor: dev(avg_close, curr.close),
                    profit,
                });
            }
        }
        qd.push(index.datetime, &mut items);
        items.clear();
    }

    qd.raw_value()
}
