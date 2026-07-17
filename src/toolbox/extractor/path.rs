use derive_more::{Deref, DerefMut};
use salvo::{Depot, Extractible, Request, extract::Metadata};
use serde::Deserialize;
use validator::Validate;

use super::validate;
use crate::resp::Res;

static METADATA: Metadata = Metadata::new("Path");

/// 提取路径参数。
#[derive(Debug, Deref, DerefMut)]
pub struct Path<T>(pub T);

impl<'ex, T> Extractible<'ex> for Path<T>
where
    T: Deserialize<'ex>,
{
    fn metadata() -> &'static Metadata {
        &METADATA
    }

    #[allow(refining_impl_trait)]
    async fn extract(req: &'ex mut Request, _depot: &'ex mut Depot) -> Result<Self, Res> {
        req.parse_params().map(Self).map_err(Into::into)
    }
}

/// 提取路径参数并校验。
#[derive(Debug, Deref, DerefMut)]
pub struct VPath<T>(pub T);

impl<'ex, T> Extractible<'ex> for VPath<T>
where
    T: Deserialize<'ex> + Validate,
{
    fn metadata() -> &'static Metadata {
        &METADATA
    }

    #[allow(refining_impl_trait)]
    async fn extract(req: &'ex mut Request, _depot: &'ex mut Depot) -> Result<Self, Res> {
        let data = req.parse_params().map_err(Res::from)?;
        validate(&data)?;
        Ok(Self(data))
    }
}
