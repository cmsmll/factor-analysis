pub mod mode1;

use std::{collections::HashSet, sync::Arc};

use salvo::prelude::*;
use serde_json::value::RawValue;
use time::macros::date;

use crate::{
    CACHE, DF, LIST,
    model::{QuantileData, TempItem},
    reject, res, resolve,
    resp::{Res, Resp},
};

pub async fn router() -> Router {
    println!("股票池数量: {}", DF.list.len());
    println!("开始时间: {}", DF.start);
    println!("结束时间: {}", DF.end);
    Router::new()
        .push(
            Router::with_path("api")
                .push(mode1::mode1_router().await)
                .push(Router::with_path("indice").get(indice))
                .push(Router::with_path("sector").get(sector))
                .push(Router::with_path("list").get(list))
                .push(Router::with_path("test").get(test)),
        )
        .get(hello)
}

#[handler]
fn list() -> Res<Box<RawValue>> {
    let value = LIST.lock().unwrap();
    let value = serde_json::to_string(&*value).unwrap();
    let value = RawValue::from_string(value).unwrap();
    res!(value => 200, "ok")
}

#[handler]
fn indice() -> Res<Arc<HashSet<String>>> {
    res!(DF.indice.clone() => 200, "ok")
}

#[handler]
fn sector() -> Res<Arc<HashSet<String>>> {
    res!(DF.sector.clone() => 200, "ok")
}

#[handler]
async fn hello() -> Resp<&'static str> {
    resolve!("Hello World" => 200, "ok")
}

#[handler]
async fn test() -> Resp<Arc<RawValue>> {
    match CACHE.get_or_run(Arc::from("test"), test_run).await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

fn test_run() -> Box<RawValue> {
    let df = DF.range(date!(2025 - 01 - 01), date!(2025 - 12 - 31));
    let mut qd: QuantileData = QuantileData::new("测试换手率因子", "", 5);
    for index in df.index_iter() {
        let mut items = Vec::with_capacity(df.list.len());
        for item in &df.list {
            if let Some((curr, _)) = item.data(&index)
                && let Some(next1) = item.after(&index, 1)
                && let Some(next2) = item.after(&index, 2)
            {
                let profit1 = (next1.close - curr.close) / curr.close;
                let profit2 = (next1.close - next1.open) / next1.open;
                let profit3 = (next2.open - next1.open) / next1.open;
                let profit4 = (next2.close - next1.open) / next1.open;

                items.push(TempItem {
                    profit1,
                    profit2,
                    profit3,
                    profit4,
                    name: item.metadata.name.clone(),
                    code: item.metadata.code.clone(),
                    factor: curr.turnover_rate,
                });
            }
        }
        qd.push(index.datetime, items);
    }

    qd.raw_value()
}
