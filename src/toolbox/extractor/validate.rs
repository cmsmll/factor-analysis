use std::fmt::Write;

use validator::Validate;

use crate::resp::Res;

/// 校验数据，并将字段错误转换为统一响应。
pub fn validate(data: &(impl Validate + ?Sized)) -> Result<(), Res> {
    let Err(errors) = data.validate() else {
        return Ok(());
    };

    let mut message = String::from("数据验证失败: ");
    for (name, fields) in errors.field_errors() {
        let codes = fields.iter().map(|field| field.code.as_ref()).collect::<Vec<_>>().join(", ");
        let _ = write!(message, "{name}<{codes}>; ");
    }
    message.truncate(message.trim_end_matches("; ").len());

    Err(Res::msg(422, message))
}
