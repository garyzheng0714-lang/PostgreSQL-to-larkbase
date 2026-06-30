//! PG 文本值 → Bitable 协议值格式化（对应 Python `formatter.py`）。
//!
//! 输入是 PG 文本协议的单元格字符串（NULL 为 None）。绝不 panic：任何无法解析
//! 的值回退为 null 或原文本。日期统一折算为 Unix 毫秒（UTC）。

use chrono::{DateTime, NaiveDate, NaiveDateTime};
use serde_json::{Number, Value};

use super::type_map::{FIELD_CHECKBOX, FIELD_CURRENCY, FIELD_DATE, FIELD_NUMBER};

/// PG 文本值 → Bitable 值。null 对任意字段类型返回 null。
pub fn format_cell(text: Option<&str>, field_type: i32) -> Value {
    let s = match text {
        Some(s) => s,
        None => return Value::Null,
    };
    match field_type {
        // 文本 / 单选 / 多选 / 条码 / 电话 / 超链接 / 地理位置
        1 | 3 | 4 | 6 | 9 | 10 | 13 => Value::String(s.to_string()),
        FIELD_NUMBER => format_number(s),
        FIELD_DATE => format_date_ms(s),
        FIELD_CHECKBOX => Value::Bool(parse_bool(s)),
        FIELD_CURRENCY => format_currency(s),
        _ => Value::String(s.to_string()),
    }
}

/// 主键拼接用文本（对应 Python `str(value)`，NULL → "None"）。
pub fn pk_text(text: Option<&str>) -> String {
    text.map(|s| s.to_string()).unwrap_or_else(|| "None".into())
}

fn json_num(f: f64) -> Value {
    if f.is_finite() {
        Number::from_f64(f).map(Value::Number).unwrap_or(Value::Null)
    } else {
        Value::Null
    }
}

fn format_number(s: &str) -> Value {
    // 整数文本优先用 i64（避免 f64 精度损失）
    if let Ok(i) = s.parse::<i64>() {
        return Value::Number(i.into());
    }
    match s.parse::<f64>() {
        Ok(f) if f.is_finite() => {
            // 整数值（如 "42.00"）输出整数，与 Python Decimal 一致
            if f.fract() == 0.0 && f.abs() < i64::MAX as f64 {
                Value::Number((f as i64).into())
            } else {
                json_num(f)
            }
        }
        _ => Value::Null, // NaN/Inf/不可解析
    }
}

fn parse_bool(s: &str) -> bool {
    matches!(s, "t" | "true" | "True" | "TRUE" | "1" | "yes" | "y")
}

fn format_currency(s: &str) -> Value {
    let cleaned: String = s.chars().filter(|c| *c != '$' && *c != ',').collect();
    match cleaned.trim().parse::<f64>() {
        Ok(f) => json_num(f),
        Err(_) => Value::Null,
    }
}

/// PG 日期/时间戳文本 → Unix 毫秒（UTC）。
/// 支持 timestamptz（带偏移）、timestamp（无偏移，按 UTC 处理）、date。
fn format_date_ms(s: &str) -> Value {
    let s = s.trim();
    // 1) 带时区偏移的 timestamptz，如 "2024-01-01 12:00:00+00" / "...123+08"
    for fmt in ["%Y-%m-%d %H:%M:%S%.f%#z", "%Y-%m-%dT%H:%M:%S%.f%#z"] {
        if let Ok(dt) = DateTime::parse_from_str(s, fmt) {
            return Value::Number(dt.timestamp_millis().into());
        }
    }
    // 2) 无时区 timestamp，按 UTC 解释
    for fmt in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Value::Number(ndt.and_utc().timestamp_millis().into());
        }
    }
    // 3) 纯日期，UTC 午夜
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        if let Some(ndt) = d.and_hms_opt(0, 0, 0) {
            return Value::Number(ndt.and_utc().timestamp_millis().into());
        }
    }
    Value::Null
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn null_returns_null() {
        assert_eq!(format_cell(None, 1), Value::Null);
        assert_eq!(format_cell(None, 2), Value::Null);
        assert_eq!(format_cell(None, 5), Value::Null);
    }

    #[test]
    fn text_passthrough() {
        assert_eq!(format_cell(Some("hello"), 1), json!("hello"));
    }

    #[test]
    fn number_integer_and_float() {
        assert_eq!(format_cell(Some("42"), 2), json!(42));
        assert_eq!(format_cell(Some("42.00"), 2), json!(42));
        assert_eq!(format_cell(Some("42.5"), 2), json!(42.5));
        assert_eq!(format_cell(Some("-13"), 2), json!(-13));
    }

    #[test]
    fn number_nan_to_null() {
        assert_eq!(format_cell(Some("NaN"), 2), Value::Null);
    }

    #[test]
    fn bool_parsing() {
        assert_eq!(format_cell(Some("t"), 7), json!(true));
        assert_eq!(format_cell(Some("f"), 7), json!(false));
    }

    #[test]
    fn currency_cleaning() {
        assert_eq!(format_cell(Some("$1,234.56"), 8), json!(1234.56));
        assert_eq!(format_cell(Some("1000"), 8), json!(1000.0));
    }

    #[test]
    fn date_only_to_ms() {
        // 2024-01-01 00:00:00 UTC = 1704067200000
        assert_eq!(format_cell(Some("2024-01-01"), 5), json!(1704067200000i64));
    }

    #[test]
    fn timestamptz_to_ms() {
        // 2023-10-22 16:34:54 UTC = 1697992494000
        assert_eq!(
            format_cell(Some("2023-10-22 16:34:54+00"), 5),
            json!(1697992494000i64)
        );
    }

    #[test]
    fn timestamp_naive_utc() {
        assert_eq!(
            format_cell(Some("2024-01-01 00:00:00"), 5),
            json!(1704067200000i64)
        );
    }

    #[test]
    fn timestamptz_with_offset_normalizes_utc() {
        // 2024-01-01 08:00:00+08 == 2024-01-01 00:00:00 UTC
        assert_eq!(
            format_cell(Some("2024-01-01 08:00:00+08"), 5),
            json!(1704067200000i64)
        );
    }

    #[test]
    fn pk_text_handles_null() {
        assert_eq!(pk_text(None), "None");
        assert_eq!(pk_text(Some("123")), "123");
    }
}
