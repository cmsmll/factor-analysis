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

use std::sync::LazyLock;

pub use app::{App, ParseCommand, RunCommand, TestCommand};
pub use toolbox::*;

use crate::{
    config::Config,
    db::{DataFrame, DataFrameDb},
    router::mode1::manager::Mode1Manager,
};

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load_or_gen_default);
pub static DF: LazyLock<DataFrame> = LazyLock::new(|| DataFrameDb::from_config(&CONFIG).unwrap().query_all().unwrap());
pub static MODE1: LazyLock<Mode1Manager> = LazyLock::new(|| Mode1Manager::new(&CONFIG.data.cache));
