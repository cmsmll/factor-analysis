use validator::Validate;

use crate::resp::Res;

/// 校验数据，并将字段错误转换为统一响应。
pub fn validate(data: &(impl Validate + ?Sized)) -> Result<(), Res> {
    let Err(errors) = data.validate() else {
        return Ok(());
    };

    Err(Res::msg(422, format!("数据验证失败: {errors}")))
}
