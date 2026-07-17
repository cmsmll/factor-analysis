use derive_more::{Deref, DerefMut};
use salvo::{Depot, Extractible, Request, extract::Metadata};
use serde::Deserialize;
use validator::Validate;

use super::validate;
use crate::resp::Res;

static METADATA: Metadata = Metadata::new("Form");

/// 提取表单请求数据。
#[derive(Debug, Deref, DerefMut)]
pub struct Form<T>(pub T);

impl<'ex, T> Extractible<'ex> for Form<T>
where
    T: Deserialize<'ex>,
{
    fn metadata() -> &'static Metadata {
        &METADATA
    }

    #[allow(refining_impl_trait)]
    async fn extract(req: &'ex mut Request, _depot: &'ex mut Depot) -> Result<Self, Res> {
        req.parse_form().await.map(Self).map_err(Into::into)
    }
}

/// 提取表单请求数据并校验。
#[derive(Debug, Deref, DerefMut)]
pub struct VForm<T>(pub T);

impl<'ex, T> Extractible<'ex> for VForm<T>
where
    T: Deserialize<'ex> + Validate,
{
    fn metadata() -> &'static Metadata {
        &METADATA
    }

    #[allow(refining_impl_trait)]
    async fn extract(req: &'ex mut Request, _depot: &'ex mut Depot) -> Result<Self, Res> {
        let data = req.parse_form().await.map_err(Res::from)?;
        validate(&data)?;
        Ok(Self(data))
    }
}
