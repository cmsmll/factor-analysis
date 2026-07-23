//! 振幅因子接口。

use std::sync::Arc;

use salvo::{Router, Writer};
use salvo_oapi::{ToSchema, endpoint};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Receiver;

use crate::{math::dev, prelude::*, router::mode1::Base, toolbox::VJson};

/// 振幅因子分析请求。
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
        let recv = CACHE.get_or_run(key, move || amplitude_run(req));
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

/// 注册振幅因子接口，并准备默认请求模板和默认结果缓存。
///
/// # Route
///
/// `POST /api/mode1/{factor_id}`
///
/// 初始化路由时会把默认 [`Req`] 写入模式一接口列表，并预先计算默认参数结果。
/// `factor_id` 为 [`Req::id`] 生成的动态值，客户端应通过
/// `POST /api/mode1/list` 获取。
pub async fn router() -> Router {
    MODE1.register(Arc::new(Req::register)).await;
    Router::with_path(Req::id()).post(amplitude)
}

/// 执行振幅因子的分位分析。
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
/// 每个交易日使用 `(当日最高价 - 当日最低价) / 当日最低价` 计算振幅，
/// 按振幅从低到高排序并切分为 `base.count` 个分位。最低价为 0 时，
/// 振幅按 0 处理。股票数少于分位数时，所有分位共享当日完整股票集合。
///
/// # Response
///
/// 成功时返回 `200`，`data` 为 [`QuantileData`]。JSON 解析失败或请求头错误
/// 由提取器返回 `415`；后台分析任务失败时返回 `400` 和 `"获取数据失败"`。
#[endpoint(
    tags("模式一"),
    operation_id = "analyze_amplitude",
    responses(
        (status_code = 200, description = "振幅因子分析结果", body = Res<QuantileData>),
        (status_code = 400, description = "分析任务失败", body = Res<()>),
        (status_code = 422, description = "参数校验失败", body = Res<()>),
        (status_code = 415, description = "Content-Type 或 JSON 请求体错误", body = Res<()>),
    )
)]
pub async fn amplitude(args: VJson<Req>) -> Resp<Arc<RawValue>> {
    let key = args.0.hashcode();
    match CACHE.get_or_run(key, move || amplitude_run(args.0)).recv().await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

/// 根据请求参数计算振幅分位数据。
///
/// 只有同时具备当日、下一交易日和下下交易日行情的股票才参与当日计算。
/// 四种收益依次为：当日收盘到下一日收盘、下一日开盘到收盘、下一日开盘到
/// 下下日开盘、下一日开盘到下下日收盘。
fn amplitude_run(args: Req) -> Box<RawValue> {
    let df = DF.filter(&args.base.filter);
    let mut qd = QuantileData::new("振幅因子", "振幅:=(HIGH-LOW)/LOW", args.base.count);
    let mut items = Vec::with_capacity(df.list.len());

    for index in df.index_iter() {
        for item in &df.list {
            if let Some((curr, profit)) = item.data(&index)
                && curr.filter_st(args.base.filter_st)
            {
                items.push(TempItem {
                    factor: amplitude_factor(curr.high, curr.low),
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

/// 计算单日振幅，最低价为零时返回零。
#[inline]
fn amplitude_factor(high: f64, low: f64) -> f64 {
    dev(high - low, low)
}
