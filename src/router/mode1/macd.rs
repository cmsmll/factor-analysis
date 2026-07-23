//! 平滑异同移动平均因子。

use std::sync::Arc;

use salvo::{Router, Writer};
use salvo_oapi::{ToSchema, endpoint};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Receiver;
use validator::ValidationError;

use crate::{
    math::{MACD, dev},
    prelude::*,
    reject, resolve,
    resp::Resp,
    router::mode1::Base,
    toolbox::VJson,
};

/// MACD 核心参数。
#[derive(Debug, Serialize, Deserialize, ToSchema, validator::Validate)]
#[validate(schema(function = "validate_core"))]
pub struct Core {
    /// 短线周期。
    #[validate(custom(function = "super::validate_period"))]
    pub short: UntArg,
    /// 长线周期。
    #[validate(custom(function = "super::validate_period"))]
    pub long: UntArg,
    /// 中线周期。
    #[validate(custom(function = "super::validate_period"))]
    pub mid: UntArg,
}

fn validate_core(core: &Core) -> Result<(), ValidationError> {
    if core.short.value < core.long.value {
        Ok(())
    } else {
        Err(ValidationError::new("short_less_than_long").with_message("短线周期必须小于长线周期".into()))
    }
}

impl Core {
    pub fn new(short: usize, long: usize, mid: usize) -> Self {
        assert!(short >= 2, "短线周期必须大于等于 2");
        assert!(long >= 2, "长线周期必须大于等于 2");
        assert!(mid >= 2, "中线周期必须大于等于 2");
        assert!(short < long, "短线周期必须小于长线周期");

        Self {
            short: UntArg::new("短线", short),
            long: UntArg::new("长线", long),
            mid: UntArg::new("中线", mid),
        }
    }
}

/// 平滑异同移动平均因子分析请求。
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
        let recv = MODE1.cache.get_or_run(key, move || macd_run(req));
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
            core: Core::new(12, 26, 9),
        }
    }
}

/// 注册标准参数 `SHORT=12, LONG=26, MID=9` 的 MACD 因子。
pub async fn router() -> Router {
    MODE1.register(Arc::new(Req::register)).await;
    Router::with_path(Req::id()).post(macd)
}

/// 按 MACD 柱值与今日收盘价的比值进行分位分析。
#[endpoint(
    tags("模式一"),
    operation_id = "analyze_macd",
    responses(
        (status_code = 200, description = "平滑异同移动平均因子分析结果", body = Res<QuantileData>),
        (status_code = 400, description = "分析任务失败", body = Res<()>),
        (status_code = 422, description = "参数校验失败", body = Res<()>),
        (status_code = 415, description = "Content-Type 或 JSON 请求体错误", body = Res<()>),
    )
)]
pub async fn macd(args: VJson<Req>) -> Resp<Arc<RawValue>> {
    let key = args.0.hashcode();
    match MODE1.cache.get_or_run(key, move || macd_run(args.0)).recv().await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

fn macd_run(args: Req) -> Box<RawValue> {
    let short = args.core.short.value;
    let long = args.core.long.value;
    let mid = args.core.mid.value;
    let df = DF.filter(&args.base.filter);
    let mut qd = QuantileData::new(
        "平滑异同移动平均因子(MACD)",
        format!(
            "DIF:=EMA(CLOSE,SHORT)-EMA(CLOSE,LONG); DEA:=EMA(DIF,MID); MACD:=2*(DIF-DEA); FACTOR:=MACD/CLOSE; SHORT:={short}; LONG:={long}; MID:={mid}"
        ),
        args.base.count,
    );
    let mut items = Vec::with_capacity(df.list.len());
    let mut store = vec![MACD::new(short, long, mid); df.list.len()];

    for index in df.index_iter() {
        for (item, store) in df.list.iter().zip(store.iter_mut()) {
            if let Some((curr, profit)) = item.data(&index)
                && curr.filter_st(args.base.filter_st)
                && let Some(macd_value) = store.next(curr.close)
            {
                items.push(TempItem {
                    factor: dev(macd_value, curr.close),
                    profit,
                });
            }
        }
        qd.push(index.datetime, &mut items);
        items.clear();
    }

    qd.raw_value()
}
