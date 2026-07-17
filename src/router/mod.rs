pub mod mode1;

use std::{collections::HashSet, sync::Arc};

use salvo::prelude::*;
use serde_json::value::RawValue;
use time::macros::date;

use crate::{
    CACHE, DF,
    model::{Item, QuantileData},
    reject, res, resolve,
    resp::{Res, Resp},
};

pub fn api_router() -> Router {
    Router::with_path("api")
        .push(Router::with_path("indice").get(indice))
        .push(Router::with_path("sector").get(sector))
        .push(Router::with_path("test").get(test))
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

                items.push(Item {
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
