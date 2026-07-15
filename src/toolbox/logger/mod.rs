pub mod message;
pub mod middleware;
pub mod output;

use time::OffsetDateTime;

pub use middleware::*;

pub(crate) fn now() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}
