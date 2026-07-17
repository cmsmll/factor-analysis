use serde::{Deserialize, Deserializer, Serializer, de::Error};
use time::{Date, format_description::well_known::Iso8601};

/// 将日期序列化为 `YYYY-MM-DD` 字符串。
pub fn serialize<S>(date: &Date, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&date.to_string())
}

/// 将 `YYYY-MM-DD` 字符串反序列化为日期。
pub fn deserialize<'de, D>(deserializer: D) -> Result<Date, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    Date::parse(&value, &Iso8601::DATE).map_err(D::Error::custom)
}
