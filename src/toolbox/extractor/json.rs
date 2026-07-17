use std::sync::LazyLock;

use derive_more::{Deref, DerefMut};
use salvo::{
    Depot, Extractible, Request,
    extract::Metadata,
    http::{
        HeaderMap,
        headers::{ContentType, HeaderMapExt},
    },
};
use serde::Deserialize;
use validator::Validate;

use super::validate;
use crate::resp::Res;

static METADATA: Metadata = Metadata::new("Json");
static JSON_CONTENT_TYPE: LazyLock<ContentType> = LazyLock::new(ContentType::json);

/// 判断请求头是否为 JSON。
pub fn is_json_content(headers: &HeaderMap) -> bool {
    headers
        .typed_get::<ContentType>()
        .is_some_and(|content_type| content_type == *JSON_CONTENT_TYPE)
}

/// 提取 JSON 请求数据。
#[derive(Debug, Deref, DerefMut)]
pub struct Json<T>(pub T);

impl<'ex, T> Extractible<'ex> for Json<T>
where
    T: Deserialize<'ex>,
{
    fn metadata() -> &'static Metadata {
        &METADATA
    }

    #[allow(refining_impl_trait)]
    async fn extract(req: &'ex mut Request, _depot: &'ex mut Depot) -> Result<Self, Res> {
        req.parse_json().await.map(Self).map_err(Into::into)
    }
}

/// 提取 JSON 请求数据并校验。
#[derive(Debug, Deref, DerefMut)]
pub struct VJson<T>(pub T);

impl<'ex, T> Extractible<'ex> for VJson<T>
where
    T: Deserialize<'ex> + Validate,
{
    fn metadata() -> &'static Metadata {
        &METADATA
    }

    #[allow(refining_impl_trait)]
    async fn extract(req: &'ex mut Request, _depot: &'ex mut Depot) -> Result<Self, Res> {
        let data = req.parse_json().await.map_err(Res::from)?;
        validate(&data)?;
        Ok(Self(data))
    }
}
