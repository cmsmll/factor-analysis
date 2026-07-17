use std::sync::Arc;

use salvo::{
    Depot, Request, Response, Writer, async_trait,
    http::{HeaderValue, StatusCode, StatusError, header::CONTENT_TYPE},
};
use salvo_oapi::{Components, EndpointOutRegister, Operation, ToSchema};
use serde::Serialize;

pub type Resp<T, E = ()> = Result<Res<T>, Res<E>>;

#[derive(Debug, Serialize, ToSchema)]
pub struct Res<T = ()> {
    info: Arc<str>,
    code: u16,
    data: T,
}

impl Res<()> {
    pub fn msg(code: u16, info: impl Into<Arc<str>>) -> Self {
        Self::new(code, info, ())
    }
}

impl<T> Res<T> {
    pub fn new(code: u16, info: impl Into<Arc<str>>, data: T) -> Self {
        Self {
            code,
            info: info.into(),
            data,
        }
    }
}

impl<T> EndpointOutRegister for Res<T> {
    fn register(_components: &mut Components, _operation: &mut Operation) {}
}

#[async_trait]
impl<T: Serialize + Send> Writer for Res<T> {
    async fn write(self, _req: &mut Request, depot: &mut Depot, res: &mut Response) {
        let status = StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        match serde_json::to_vec(&self) {
            Ok(bytes) => {
                res.headers_mut()
                    .insert(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"));
                res.status_code(status);
                res.write_body(bytes).ok();
            }
            Err(_) => {
                res.render(StatusError::internal_server_error());
            }
        }

        if status.as_u16() >= 400 {
            depot.insert("error", self.info);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::{Res, Resp};
    use crate::{reject, rejectf, resolve, resolvef};

    #[test]
    fn msg_constructor_builds_empty_payload() {
        let res = Res::msg(400, "bad request");

        assert_eq!(res.code, 400);
        assert_eq!(&*res.info, "bad request");
        assert_eq!(res.data, ());
    }

    #[test]
    fn ok_macros_match_their_builder_variants() {
        let ok: Resp<&'static str> = resolve!("hello" => 200, "ok");
        let ok = ok.expect("resolve! should return Ok");
        assert_eq!(ok.code, 200);
        assert_eq!(&*ok.info, "ok");
        assert_eq!(ok.data, "hello");

        let okf: Resp<&'static str> = resolvef!("hello" => 201, "{} {}", "created", "ok");
        let okf = okf.expect("resolvef! should return Ok");
        assert_eq!(okf.code, 201);
        assert_eq!(&*okf.info, "created ok");
        assert_eq!(okf.data, "hello");
    }

    #[test]
    fn err_macros_match_their_builder_variants() {
        let err: Resp<()> = reject!(400, "bad request");
        let err = err.expect_err("reject! should return Err");
        assert_eq!(err.code, 400);
        assert_eq!(&*err.info, "bad request");
        assert_eq!(err.data, ());

        let errf: Resp<()> = rejectf!(422, "{} {}", "invalid", "payload");
        let errf = errf.expect_err("rejectf! should return Err");
        assert_eq!(errf.code, 422);
        assert_eq!(&*errf.info, "invalid payload");
        assert_eq!(errf.data, ());

        let err_with_data: Resp<(), &'static str> = reject!("details" => 422, "validation failed");
        let err_with_data = err_with_data.expect_err("reject! should support error data");
        assert_eq!(err_with_data.code, 422);
        assert_eq!(err_with_data.data, "details");
    }

    #[test]
    fn io_errors_convert_into_error_responses() {
        let err = io::Error::other("disk offline");
        let res: Res<()> = err.into();

        assert_eq!(res.code, 500);
        assert_eq!(&*res.info, "IoError: disk offline");
    }
}
