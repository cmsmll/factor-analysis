//！ 换手率因子
use std::sync::Arc;

use salvo::{Router, Writer, handler};
use serde::{Deserialize, Serialize};

use crate::{prelude::*, reject, resolve, resp::Resp, toolbox::Json};

#[derive(Debug, Serialize, Deserialize)]
pub struct Req {
    pub id: String,
    pub count: UntArg,
    pub filter: Filter,
}

impl ArgsHandle for Req {}

impl Default for Req {
    fn default() -> Self {
        Self {
            id: Self::id(),
            filter: Filter::from_config(&CONFIG),
            count: UntArg::new("分位数量", 5),
        }
    }
}

pub async fn router() -> Router {
    let req = Req::default();
    let key = req.hashcode();
    LIST.lock().unwrap().push(req.raw_value());
    CACHE.get_or_run(key, move || turnover_rate_run(req)).await.unwrap();
    Router::with_path(Req::id()).post(turnover_rate)
}

#[handler]
pub async fn turnover_rate(args: Json<Req>) -> Resp<Arc<RawValue>> {
    let key = args.0.hashcode();
    match CACHE.get_or_run(key, move || turnover_rate_run(args.0)).await {
        Ok(res) => resolve!(res => 200, "ok"),
        Err(_) => reject!(400, "获取数据失败"),
    }
}

fn turnover_rate_run(args: Req) -> Box<RawValue> {
    let df = DF.filter(&args.filter);
    let mut qd: QuantileData = QuantileData::new("换手率因子", "按换手率从低到高分位", *args.count);

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
