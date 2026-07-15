/// 快速构建响应。
///
/// # Examples
///
/// ```no_run
/// use factor_analysis::res;
///
/// res!(200, "ok");
/// res!("hello" => 200, "ok");
/// ```
#[macro_export]
macro_rules! res {
    ($code:expr, $info:expr) => {
        $crate::resp::Res::msg($code, $info)
    };
    ($data:expr => $code:expr, $info:expr) => {
        $crate::resp::Res::new($code, $info, $data)
    };
}

/// 快速构建支持格式化信息的响应。
///
/// # Examples
///
/// ```no_run
/// use factor_analysis::resf;
///
/// let name = "factor";
/// resf!(400, "invalid field: {name}");
/// resf!("Hello World" => 200, "hello {name}");
/// ```
#[macro_export]
macro_rules! resf {
    ($code:expr, $($msg:tt)+) => {
        $crate::resp::Res::msg($code, format!($($msg)+))
    };
    ($data:expr => $code:expr, $($msg:tt)+) => {
        $crate::resp::Res::new($code, format!($($msg)+), $data)
    };
}

/// 快速返回不带格式化信息的错误。
///
/// # Examples
///
/// ```no_run
/// use factor_analysis::{reject, resp::Resp};
///
/// let _: Resp<()> = reject!(400, "bad request");
/// let _: Resp<(), &str> = reject!("details" => 422, "validation failed");
/// ```
#[macro_export]
macro_rules! reject {
    ($($t:tt)*) => {
        Err($crate::res!($($t)*))
    };
}

/// 快速返回支持格式化信息的错误。
///
/// # Examples
///
/// ```no_run
/// use factor_analysis::{rejectf, resp::Resp};
///
/// let field = "name";
/// let _: Resp<()> = rejectf!(400, "missing field: {field}");
/// let _: Resp<(), &str> = rejectf!("details" => 422, "invalid field: {field}");
/// ```
#[macro_export]
macro_rules! rejectf {
    ($($t:tt)*) => {
        Err($crate::resf!($($t)*))
    };
}

/// 快速返回不带格式化信息的成功。
///
/// # Examples
///
/// ```no_run
/// use factor_analysis::{resolve, resp::Resp};
///
/// let _: Resp<()> = resolve!(200, "ok");
/// let _: Resp<&str> = resolve!("hello" => 200, "ok");
/// ```
#[macro_export]
macro_rules! resolve {
    ($($t:tt)*) => {
        Ok($crate::res!($($t)*))
    };
}

/// 快速返回支持格式化信息的成功。
///
/// # Examples
///
/// ```no_run
/// use factor_analysis::{resolvef, resp::Resp};
///
/// let count = 3;
/// let _: Resp<()> = resolvef!(200, "loaded {count} items");
/// let _: Resp<&str> = resolvef!("hello" => 200, "loaded {count} items");
/// ```
#[macro_export]
macro_rules! resolvef {
    ($($t:tt)*) => {
        Ok($crate::resf!($($t)*))
    };
}
