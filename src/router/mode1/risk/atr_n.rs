//! 波动率因子（基于真实波幅 ATR）接口。

use std::sync::Arc;

use salvo::{Router, Writer};
use salvo_oapi::{ToSchema, endpoint};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Receiver;

use crate::{math::{SMA, dev}, prelude::*, reject, resolve, resp::Resp, router::mode1::Base, toolbox::VJson};

/// 波动率因子的核心参数。
#[derive(Debug, Serialize, Deserialize, ToSchema, validator::Validate)]
pub struct Core {
    /// 平滑周期，单位为交易日。
    #[validate(custom(function = "super::super::validate_period"))]
    pub period: UntArg,
}

impl Core {
    pub fn new(period: usize) -> Self {
        Self {
            period: UntArg::new("ATR平滑周期", period),
        }
    }
}

/// 波动率因子分析请求。
///
/// 客户端通常先从 `POST /api/mode1/list` 取得默认结构，再按需修改参数。
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
        let recv = MODE1.cache.get_or_run(key, move || atr_n_run(req));
        (value, recv)
    }
}

impl ArgsHandle for Req {}

impl Default for Req {
    fn default() -> Self {
        Self::new(5)
    }
}

/// 注册 5 日、10 日和 20 日波动率因子接口，并加入模式一因子列表。
///
/// # Route
///
/// `POST /api/mode1/{factor_id}`
///
/// 初始化路由时会把默认 [`Req`] 写入模式一接口列表，并预先计算默认参数结果。
/// `factor_id` 为 [`Req::id`] 生成的动态值，客户端应通过
/// `POST /api/mode1/list` 获取。
pub async fn router() -> Router {
    MODE1.register(Arc::new(|filter| Req::register(filter, 5))).await;
    MODE1.register(Arc::new(|filter| Req::register(filter, 10))).await;
    MODE1.register(Arc::new(|filter| Req::register(filter, 20))).await;
    Router::new().push(Router::with_path(Req::id()).post(atr_n))
}

/// 执行波动率因子的分位分析。
///
/// # Route
///
/// `POST /api/mode1/{factor_id}`
///
/// 请求头必须包含 `Content-Type: application/json`。请求体使用 [`Req`]，
/// 其中 `base` 包含动态接口 ID、分位数量和股票池筛选条件。
///
/// # Analysis
///
/// 每个交易日计算各股票的真实波幅（True Range）及其 N 日简单移动平均：
///
/// ```text
/// AA = (HIGH - LOW) / LOW
/// BB = ABS(REF(CLOSE, 1) - HIGH) / REF(CLOSE, 1)
/// CC = ABS(REF(CLOSE, 1) - LOW) / REF(CLOSE, 1)
/// TR = MAX(AA, BB, CC)
/// ATR = SMA(TR, N)
/// ```
///
/// 按 ATR 从低到高排序并切分为 `base.count` 个分位。前一交易日收盘价无效时
/// BB、CC 按 0 处理。股票数少于分位数时，所有分位共享当日完整股票集合。
///
/// # Response
///
/// 成功时返回 `200`，`data` 为 [`Mode1Data`]。JSON 解析失败或请求头错误
/// 由提取器返回 `415`；后台分析任务失败时返回 `400` 和 `"获取数据失败"`。
#[endpoint(
    tags("模式一"),
    operation_id = "analyze_atr_n",
    responses(
        (status_code = 200, description = "波动率因子分析结果", body = Res<Mode1Data>),
        (status_code = 400, description = "分析任务失败", body = Res<()>),
        (status_code = 422, description = "参数校验失败", body = Res<()>),
        (status_code = 415, description = "Content-Type 或 JSON 请求体错误", body = Res<()>),
    )
)]
pub async fn atr_n(args: VJson<Req>) -> Resp<Arc<RawValue>> {
    let key = args.0.hashcode();
    match MODE1.cache.get_or_run(key, move || atr_n_run(args.0)).recv().await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

/// 根据请求参数计算 ATR 波动率分位数据。
///
/// 每只股票使用独立的 SMA 状态机累积 TR。仅当 SMA 预热完成后才参与当日排序。
/// 四种收益依次为：当日收盘到下一日收盘、下一日开盘到收盘、下一日开盘到下下日开盘、
/// 下一日开盘到下下日收盘。
fn atr_n_run(args: Req) -> Box<RawValue> {
    let period = args.core.period.value;
    let df = DF.filter(&args.base.filter);
    let mut qd = Mode1Data::new(
        format!("波动率因子(ATR){period}日"),
        format!("TR:=MAX((H-L)/L,ABS(REF(C,1)-H)/REF(C,1),ABS(REF(C,1)-L)/REF(C,1)); ATR:=SMA(TR,{period})"),
        super::LABEL,
        args.base.count,
    );
    let mut items = Vec::with_capacity(df.list.len());
    let mut stores: Vec<SMA> = (0..df.list.len()).map(|_| SMA::new(period.max(2))).collect();

    for index in df.index_iter() {
        for (store, item) in stores.iter_mut().zip(df.list.iter()) {
            if let Some((curr, profit)) = item.data(&index)
                && curr.filter_st(args.base.filter_st)
                && let Some(prev1) = item.before(&index, 1)
            {
                let tr = true_range(curr.high, curr.low, prev1.close);
                if let Some(factor) = store.next(tr) {
                    items.push(Mode1Temp { factor, profit });
                }
            }
        }
        qd.push(index.datetime, &mut items);
        items.clear();
    }

    qd.raw_value()
}

/// 计算单日真实波幅 TR。
///
/// `AA = (HIGH - LOW) / LOW`
/// `BB = ABS(REF(CLOSE, 1) - HIGH) / REF(CLOSE, 1)`
/// `CC = ABS(REF(CLOSE, 1) - LOW) / REF(CLOSE, 1)`
/// `TR = MAX(AA, BB, CC)`
///
/// 前一交易日收盘价无效（≤ 0）时 BB 和 CC 按 0 处理。
#[inline]
fn true_range(high: f64, low: f64, prev_close: f64) -> f64 {
    let aa = dev(high - low, low);
    let bb = if prev_close > 0.0 { dev((prev_close - high).abs(), prev_close) } else { 0.0 };
    let cc = if prev_close > 0.0 { dev((prev_close - low).abs(), prev_close) } else { 0.0 };
    aa.max(bb).max(cc)
}
