//! 终极指标因子。

use std::sync::Arc;

use salvo::{Router, Writer};
use salvo_oapi::{ToSchema, endpoint};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Receiver;

use crate::{math::TRIX, prelude::*, reject, resolve, resp::Resp, router::mode1::Base, toolbox::VJson};

/// TRIX 的三重指数移动平均周期。
#[derive(Debug, Serialize, Deserialize, ToSchema, validator::Validate)]
pub struct Core {
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

/// 终极指标因子分析请求。
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
        let recv = MODE1.cache.get_or_run(key, move || trix_n_run(req));
        (value, recv)
    }
}

impl ArgsHandle for Req {}

/// 注册 10 日终极指标因子模板。
pub async fn router() -> Router {
    MODE1.register(Arc::new(|filter| Req::register(filter, 10))).await;
    Router::with_path(Req::id()).post(trix_n)
}

/// 按三重指数移动平均线的单日变化率进行分位分析。
#[endpoint(
    tags("模式一"),
    operation_id = "analyze_trix_n",
    responses(
        (status_code = 200, description = "终极指标因子分析结果", body = Res<QuantileData>),
        (status_code = 400, description = "分析任务失败", body = Res<()>),
        (status_code = 422, description = "参数校验失败", body = Res<()>),
        (status_code = 415, description = "Content-Type 或 JSON 请求体错误", body = Res<()>),
    )
)]
pub async fn trix_n(args: VJson<Req>) -> Resp<Arc<RawValue>> {
    let key = args.0.hashcode();
    match MODE1.cache.get_or_run(key, move || trix_n_run(args.0)).recv().await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

fn trix_n_run(args: Req) -> Box<RawValue> {
    let period = args.core.period.value;
    let df = DF.filter(&args.base.filter);
    let mut qd = QuantileData::new(
        format!("终极指标因子(TRIX){period}日"),
        format!("MTR:=EMA(EMA(EMA(CLOSE,N),N),N); TRIX:=(MTR-REF(MTR,1))/REF(MTR,1)*100; N:={period}"),
        args.base.count,
    );
    let mut items = Vec::with_capacity(df.list.len());
    let mut store = vec![TRIX::new(period); df.list.len()];

    for index in df.index_iter() {
        for (item, store) in df.list.iter().zip(store.iter_mut()) {
            if let Some((curr, profit)) = item.data(&index)
                && curr.filter_st(args.base.filter_st)
                && let Some(factor) = store.next(curr.close)
            {
                items.push(TempItem { factor, profit });
            }
        }
        qd.push(index.datetime, &mut items);
        items.clear();
    }

    qd.raw_value()
}
