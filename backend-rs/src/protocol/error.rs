//! 飞书连接器协议错误：8 个内部错误 → 5 个飞书错误码 + 中英双语 msg。
//!
//! 关键不变量（对应 Python `error_handler.py` / `error.py`）：
//! **所有协议层错误一律返回 HTTP 200**，body 为 `{code, msg, data:null}`，
//! `msg` 是 `{"zh":..,"en":..}` 的 JSON 字符串。仅进程级故障才允许非 200。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

// 飞书标准错误码
const ERR_CONFIG: i64 = 1254400;
const ERR_AUTH: i64 = 1254403;
const ERR_SYSTEM: i64 = 1254500;
// 1254501（限流）、1254505（付费）当前未使用。

/// 统一连接器错误。每个变体对应一个飞书错误码与双语提示。
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConnectorError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("query timeout: {0}")]
    QueryTimeout(String),
    #[error("invalid sql: {0}")]
    InvalidSql(String),
    #[error("invalid page token")]
    InvalidPageToken,
    #[error("signature invalid")]
    SignatureInvalid,
    #[error("too many fields")]
    TooManyFields,
    #[error("response too large")]
    ResponseTooLarge,
    #[error("table not found")]
    TableNotFound,
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("unknown error: {0}")]
    Unknown(String),
}

impl ConnectorError {
    /// 返回 (飞书 code, 中文, 英文)。
    fn parts(&self) -> (i64, &'static str, &'static str) {
        match self {
            Self::ConnectionFailed(_) => (
                ERR_CONFIG,
                "数据库连接失败，请检查连接信息",
                "Database connection failed, please check connection info",
            ),
            Self::QueryTimeout(_) => (
                ERR_SYSTEM,
                "查询超时，请优化 SQL 或减少数据量",
                "Query timeout, please optimize SQL or reduce data volume",
            ),
            Self::InvalidSql(_) => (ERR_CONFIG, "SQL 语法错误", "SQL syntax error"),
            Self::InvalidPageToken => (
                ERR_CONFIG,
                "分页状态无效，请重新发起同步",
                "Invalid page token, please restart sync",
            ),
            Self::SignatureInvalid => (
                ERR_AUTH,
                "请求签名验证失败",
                "Request signature verification failed",
            ),
            Self::TooManyFields => (
                ERR_CONFIG,
                "字段数超过上限(299)，请减少选择的字段",
                "Field count exceeds limit (299), please reduce selected fields",
            ),
            Self::ResponseTooLarge => (
                ERR_SYSTEM,
                "单页数据量过大，请减少字段或数据量",
                "Page payload is too large; reduce selected fields or data volume",
            ),
            Self::TableNotFound => (
                ERR_CONFIG,
                "指定的表不存在",
                "Specified table does not exist",
            ),
            Self::PermissionDenied(_) => (
                ERR_AUTH,
                "数据库权限不足，请检查用户权限",
                "Insufficient database permissions, please check user privileges",
            ),
            Self::Unknown(_) => (ERR_SYSTEM, "未知系统错误", "Unknown system error"),
        }
    }

    /// 飞书错误码。
    pub fn code(&self) -> i64 {
        self.parts().0
    }

    /// 双语 msg（JSON 字符串 `{"zh":..,"en":..}`）。
    pub fn msg(&self) -> String {
        let (_, zh, en) = self.parts();
        json!({ "zh": zh, "en": en }).to_string()
    }

    /// 构造协议响应体 `{code, msg, data:null}`。
    pub fn body(&self) -> serde_json::Value {
        json!({ "code": self.code(), "msg": self.msg(), "data": serde_json::Value::Null })
    }
}

impl IntoResponse for ConnectorError {
    fn into_response(self) -> Response {
        // 协议错误 → HTTP 200 + 错误体（飞书据 body.code 判定，而非 HTTP 状态）。
        (StatusCode::OK, Json(self.body())).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_map_correctly() {
        assert_eq!(ConnectorError::ConnectionFailed("x".into()).code(), 1254400);
        assert_eq!(ConnectorError::InvalidSql("x".into()).code(), 1254400);
        assert_eq!(ConnectorError::InvalidPageToken.code(), 1254400);
        assert_eq!(ConnectorError::TooManyFields.code(), 1254400);
        assert_eq!(ConnectorError::TableNotFound.code(), 1254400);
        assert_eq!(ConnectorError::SignatureInvalid.code(), 1254403);
        assert_eq!(ConnectorError::PermissionDenied("x".into()).code(), 1254403);
        assert_eq!(ConnectorError::QueryTimeout("x".into()).code(), 1254500);
        assert_eq!(ConnectorError::ResponseTooLarge.code(), 1254500);
        assert_eq!(ConnectorError::Unknown("x".into()).code(), 1254500);
    }

    #[test]
    fn msg_is_bilingual_json() {
        let m = ConnectorError::SignatureInvalid.msg();
        let v: serde_json::Value = serde_json::from_str(&m).unwrap();
        assert!(v.get("zh").is_some() && v.get("en").is_some());
    }

    #[test]
    fn body_shape_matches_protocol() {
        let b = ConnectorError::TableNotFound.body();
        assert_eq!(b["code"], 1254400);
        assert!(b["data"].is_null());
        assert!(b["msg"].is_string());
    }
}
