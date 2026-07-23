//! 换手率因子N日平均。

use std::sync::Arc;

use salvo::{Router, Writer};
use salvo_oapi::{ToSchema, endpoint};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Receiver;

use crate::{math::SMA, prelude::*, reject, resolve, resp::Resp, router::mode1::Base, toolbox::VJson};

/// 模式一因子的核心参数。
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

/// 多日平均换手率因子分析请求。
///
/// `core.period` 表示向前取多少个交易日计算平均换手率，包含当日。
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
        let recv = MODE1.cache.get_or_run(key, move || turnover_rate_n_run(req));
        (value, recv)
    }
}

impl ArgsHandle for Req {}

/// 注册 5 日和 10 日平均换手率因子接口，并加入模式一因子列表。
pub async fn router() -> Router {
    MODE1.register(Arc::new(|filter| Req::register(filter, 5))).await;
    MODE1.register(Arc::new(|filter| Req::register(filter, 10))).await;
    Router::new().push(Router::with_path(Req::id()).post(turnover_rate_n))
}

/// 执行多日平均换手率因子的分位分析。
///
/// 每个交易日按 `core.period` 日平均换手率从低到高排序，切分为 `base.count` 个分位，
/// 并返回各分位的平均因子值、平均换手率和四种平均收益。
#[endpoint(
    tags("模式一"),
    operation_id = "analyze_turnover_rate_n",
    responses(
        (status_code = 200, description = "多日平均换手率因子分析结果", body = Res<Mode1Data>),
        (status_code = 400, description = "分析任务失败", body = Res<()>),
        (status_code = 422, description = "参数校验失败", body = Res<()>),
        (status_code = 415, description = "Content-Type 或 JSON 请求体错误", body = Res<()>),
    )
)]
pub async fn turnover_rate_n(args: VJson<Req>) -> Resp<Arc<RawValue>> {
    let key = args.0.hashcode();
    match MODE1.cache.get_or_run(key, move || turnover_rate_n_run(args.0)).recv().await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

fn turnover_rate_n_run(args: Req) -> Box<RawValue> {
    let period = args.core.period.value;
    let df = DF.filter(&args.base.filter);
    let mut qd = Mode1Data::new(
        format!("换手率因子{period}日平均"),
        format!("TURNOVER_RATE_N:=MA(TURNOVER_RATE,N); N:={period}"),
        super::LABEL,
        args.base.count,
    );
    let mut items = Vec::with_capacity(df.list.len());
    let mut store = vec![SMA::new(period); df.list.len()];

    for index in df.index_iter() {
        for (item, store) in df.list.iter().zip(store.iter_mut()) {
            if let Some((curr, profit)) = item.data(&index)
                && curr.filter_st(args.base.filter_st)
                && let Some(factor) = store.next(curr.turnover_rate)
            {
                items.push(Mode1Temp { factor, profit });
            }
        }
        qd.push(index.datetime, &mut items);
        items.clear();
    }

    qd.raw_value()
}
