use std::{collections::HashSet, sync::Arc};

use serde::{Deserialize, Serialize};
use time::Date;

use crate::toolbox::date_format;

#[derive(Debug, Serialize, Deserialize)]
pub struct Filter {
    /// 开始时间
    #[serde(with = "date_format")]
    pub start: Date,
    /// 结束时间
    #[serde(with = "date_format")]
    pub end: Date,
    /// 过滤北证券
    pub filter_bz: bool,
    /// 过滤ST
    pub filter_st: bool,
    /// 行业板块
    pub sector: HashSet<String>,
    /// 指数列表
    pub indice: HashSet<String>,
}

impl Filter {
    pub fn hashcode(&self) -> Arc<str> {
        let buf = serde_json::to_vec(self).unwrap();
        let res = blake3::hash(&buf);
        Arc::from(res.to_string())
    }
}
