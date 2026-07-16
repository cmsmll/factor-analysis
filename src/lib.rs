pub mod app;
pub mod cache;
pub mod config;
pub mod db;
pub mod math;
pub mod toolbox;

use std::sync::LazyLock;

pub use app::{App, ParseCommand, RunCommand, TestCommand};
pub use toolbox::*;

use crate::cache::Cache;

pub static CACHE: LazyLock<Cache> = LazyLock::new(|| Cache::new("cache"));
