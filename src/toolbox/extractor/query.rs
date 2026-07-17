use derive_more::{Deref, DerefMut};
use salvo::{Depot, Extractible, Request, extract::Metadata};
use serde::Deserialize;
use validator::Validate;

use super::validate;
use crate::resp::Res;

static METADATA: Metadata = Metadata::new("Query");

/// 提取查询参数。
#[derive(Debug, Deref, DerefMut)]
pub struct Query<T>(pub T);

impl<'ex, T> Extractible<'ex> for Query<T>
where
    T: Deserialize<'ex>,
{
    fn metadata() -> &'static Metadata {
        &METADATA
    }

    #[allow(refining_impl_trait)]
    async fn extract(req: &'ex mut Request, _depot: &'ex mut Depot) -> Result<Self, Res> {
        req.parse_queries().map(Self).map_err(Into::into)
    }
}

/// 提取查询参数并校验。
#[derive(Debug, Deref, DerefMut)]
pub struct VQuery<T>(pub T);

impl<'ex, T> Extractible<'ex> for VQuery<T>
where
    T: Deserialize<'ex> + Validate,
{
    fn metadata() -> &'static Metadata {
        &METADATA
    }

    #[allow(refining_impl_trait)]
    async fn extract(req: &'ex mut Request, _depot: &'ex mut Depot) -> Result<Self, Res> {
        let data = req.parse_queries().map_err(Res::from)?;
        validate(&data)?;
        Ok(Self(data))
    }
}
