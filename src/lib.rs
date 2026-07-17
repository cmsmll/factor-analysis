pub mod app;
pub mod args;
pub mod cache;
pub mod config;
pub mod db;
pub mod math;
pub mod model;
pub mod prelude;
pub mod router;
pub mod toolbox;

use std::sync::{LazyLock, Mutex};

pub use app::{App, ParseCommand, RunCommand, TestCommand};
use serde_json::value::RawValue;
pub use toolbox::*;

use crate::{
    cache::Cache,
    config::Config,
    db::{DataFrame, DataFrameDb},
};

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load_or_gen_default);
pub static CACHE: LazyLock<Cache> = LazyLock::new(|| Cache::new("cache"));
pub static DF: LazyLock<DataFrame> = LazyLock::new(|| DataFrameDb::from_config(&CONFIG).unwrap().query_all().unwrap());
pub static MODE1: LazyLock<Mutex<Vec<Box<RawValue>>>> = LazyLock::new(Default::default);
