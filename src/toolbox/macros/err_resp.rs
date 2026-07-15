use std::{io, num::ParseIntError};

use crate::toolbox::resp::Res;
use salvo::http::ParseError;

macro_rules! impl_res_from_error {
    ($ty:path, $code:expr, $msg:expr) => {
        impl From<$ty> for Res<()> {
            fn from(_: $ty) -> Self {
                Res::msg($code, $msg)
            }
        }
    };

    ($ty:path, $code:expr, $fmt:expr, this) => {
        impl From<$ty> for Res<()> {
            fn from(value: $ty) -> Self {
                Res::msg($code, format!($fmt, value))
            }
        }
    };

    ($ty:path, $code:expr, $fmt:expr, $($field:ident),+ $(,)?) => {
        impl From<$ty> for Res<()> {
            fn from(value: $ty) -> Self {
                Res::msg($code, format!($fmt, $(value.$field),+))
            }
        }
    };
}

impl_res_from_error!(io::Error, 500, "IoError: {}", this);
impl_res_from_error!(ParseIntError, 400, "ParseIntError:{}", this);
impl_res_from_error!(ParseError, 415, "ParseError: {}", this);
