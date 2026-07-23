use std::{any::TypeId, collections::HashSet, sync::Arc};

use derive_more::{Deref, DerefMut};
use salvo_oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use time::Date;

use crate::{config::Config, toolbox::date_format};

pub trait ArgsHandle: Serialize + 'static {
    fn hashcode(&self) -> Arc<str> {
        let buf = serde_json::to_vec(self).unwrap();
        let res = blake3::hash(&buf);
        Arc::from(res.to_string())
    }

    fn id() -> String {
        let id = format!("{:?}", TypeId::of::<Self>());
        String::from(&id[9..id.len() - 1])
    }

    fn raw_value(&self) -> Box<RawValue> {
        let s = serde_json::to_string(self).unwrap();
        RawValue::from_string(s).unwrap()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, validator::Validate)]
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
    pub fn new(start: Date, end: Date) -> Self {
        Self {
            start,
            end,
            filter_bz: false,
            filter_st: false,
            sector: Default::default(),
            indice: Default::default(),
        }
    }

    pub fn from_config(confg: &Config) -> Self {
        let period = confg.period.first().expect("period配置至少需要一个周期");
        Self {
            start: period.start,
            end: period.end,
            filter_bz: false,
            filter_st: false,
            sector: Default::default(),
            indice: Default::default(),
        }
    }
}

/// 数字参数
#[derive(Debug, Serialize, Deserialize, Deref, DerefMut)]
pub struct NumArg {
    pub name: String,
    #[deref]
    #[deref_mut]
    pub value: f64,
}

impl NumArg {
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self { name: name.into(), value }
    }
}

/// 整数参数
#[derive(Debug, Serialize, Deserialize, Deref, DerefMut)]
pub struct IntArg {
    pub name: String,
    #[deref]
    #[deref_mut]
    pub value: i64,
}

impl IntArg {
    pub fn new(name: impl Into<String>, value: i64) -> Self {
        Self { name: name.into(), value }
    }
}

/// 整数参数
#[derive(Debug, Serialize, Deserialize, ToSchema, Deref, DerefMut)]
pub struct UntArg {
    pub name: String,
    #[deref]
    #[deref_mut]
    pub value: usize,
}

impl UntArg {
    pub fn new(name: impl Into<String>, value: usize) -> Self {
        Self { name: name.into(), value }
    }
}

/// 字符串参数
#[derive(Debug, Serialize, Deserialize, Deref, DerefMut)]
pub struct StrArg {
    pub name: String,
    #[deref]
    #[deref_mut]
    pub value: String,
}

impl StrArg {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IntArg, NumArg, StrArg};

    // 测试参数构造后可以直接解引用为对应的参数值。
    #[test]
    fn args_deref_to_values() {
        let num = NumArg::new("换手率", 0.25);
        let int = IntArg::new("分位数", 10);
        let text = StrArg::new("策略名称", "低波动");

        assert_eq!(*num, 0.25);
        assert_eq!(*int, 10);
        assert_eq!(&*text, "低波动");
    }
}
