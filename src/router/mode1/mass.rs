//! 梅斯线因子。

use std::sync::Arc;

use salvo::{Router, Writer};
use salvo_oapi::{ToSchema, endpoint};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Receiver;

use crate::{math::MASS, prelude::*, reject, resolve, resp::Resp, router::mode1::Base, toolbox::VJson};

/// MASS 的主线与信号线周期。
#[derive(Debug, Serialize, Deserialize, ToSchema, validator::Validate)]
pub struct Core {
    #[validate(custom(function = "super::validate_period"))]
    pub n1: UntArg,
    #[validate(custom(function = "super::validate_period"))]
    pub n2: UntArg,
    #[validate(custom(function = "super::validate_period"))]
    pub m: UntArg,
}

impl Core {
    pub fn new(n1: usize, n2: usize, m: usize) -> Self {
        assert!([n1, n2, m].into_iter().all(|period| period >= 2), "周期必须大于等于 2");

        Self {
            n1: UntArg::new("N1周期", n1),
            n2: UntArg::new("N2周期", n2),
            m: UntArg::new("M周期", m),
        }
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new(9, 25, 6)
    }
}

/// 梅斯线因子分析请求。
#[derive(Debug, Serialize, Deserialize, ToSchema, validator::Validate)]
pub struct Req {
    #[validate(nested)]
    base: Base,
    #[validate(nested)]
    core: Core,
}

impl Req {
    fn register(filter: &Filter) -> (Arc<RawValue>, Receiver<Arc<RawValue>>) {
        let mut req = Self::default();
        req.base.filter = filter.clone();
        let value = Arc::from(req.raw_value());
        let key = req.hashcode();
        let recv = MODE1.cache.get_or_run(key, move || mass_run(req));
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
            core: Core::default(),
        }
    }
}

/// 注册默认参数 `N1=9, N2=25, M=6` 的梅斯线因子。
pub async fn router() -> Router {
    MODE1.register(Arc::new(Req::register)).await;
    Router::with_path(Req::id()).post(mass)
}

/// 按 MASS 主线进行分位分析。
#[endpoint(
    tags("模式一"),
    operation_id = "analyze_mass",
    responses(
        (status_code = 200, description = "梅斯线因子分析结果", body = Res<QuantileData>),
        (status_code = 400, description = "分析任务失败", body = Res<()>),
        (status_code = 422, description = "参数校验失败", body = Res<()>),
        (status_code = 415, description = "Content-Type 或 JSON 请求体错误", body = Res<()>),
    )
)]
pub async fn mass(args: VJson<Req>) -> Resp<Arc<RawValue>> {
    let key = args.0.hashcode();
    match MODE1.cache.get_or_run(key, move || mass_run(args.0)).recv().await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

fn mass_run(args: Req) -> Box<RawValue> {
    let Core { n1, n2, m } = args.core;
    let (n1, n2, m) = (n1.value, n2.value, m.value);
    let df = DF.filter(&args.base.filter);
    let mut qd = QuantileData::new(
        "梅斯线因子(MASS)",
        format!("MASS:=SUM(MA(HIGH-LOW,N1)/MA(MA(HIGH-LOW,N1),N1),N2); MAMASS:=MA(MASS,M); FACTOR:=MASS; N1:={n1}; N2:={n2}; M:={m}"),
        args.base.count,
    );
    let mut items = Vec::with_capacity(df.list.len());
    let mut store = vec![MASS::new(n1, n2, m); df.list.len()];

    for index in df.index_iter() {
        for (item, store) in df.list.iter().zip(store.iter_mut()) {
            if let Some((curr, profit)) = item.data(&index)
                && curr.filter_st(args.base.filter_st)
                && let Some(value) = store.next(curr.high, curr.low)
            {
                items.push(TempItem { factor: value.mass, profit });
            }
        }
        qd.push(index.datetime, &mut items);
        items.clear();
    }

    qd.raw_value()
}
