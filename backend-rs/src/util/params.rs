//! 健壮解析飞书 connector 请求 params（对应 Python `params_parser.py`）。
//!
//! 飞书的 params 嵌套格式多变：params 可能是 JSON 字符串或对象；其中的
//! datasourceConfig 可能是字符串或对象，甚至再嵌套一层 datasourceConfig。

use serde_json::Value;

use crate::protocol::request::DatasourceConfig;
use crate::protocol::ConnectorError;

/// 若为字符串则按 JSON 解析，否则原样返回。
fn safe_json(v: &Value) -> Value {
    if let Some(s) = v.as_str() {
        serde_json::from_str(s).unwrap_or(Value::Null)
    } else {
        v.clone()
    }
}

/// 从飞书请求体提取 `DatasourceConfig` 与完整 params 对象。
pub fn parse_feishu_params(payload: &Value) -> Result<(DatasourceConfig, Value), ConnectorError> {
    let raw_params = payload
        .get("params")
        .cloned()
        .unwrap_or_else(|| Value::String("{}".into()));
    let params = safe_json(&raw_params);
    if !params.is_object() {
        return Err(ConnectorError::ConnectionFailed(
            "params is not an object".into(),
        ));
    }

    let ds_raw = params
        .get("datasourceConfig")
        .cloned()
        .unwrap_or_else(|| Value::String("{}".into()));
    let mut ds = safe_json(&ds_raw);

    // 解包可能的嵌套 datasourceConfig
    if ds.get("datasourceConfig").is_some() {
        ds = safe_json(&ds["datasourceConfig"]);
    }
    if !ds.is_object() {
        return Err(ConnectorError::ConnectionFailed(
            "datasourceConfig is not an object".into(),
        ));
    }

    let mut config: DatasourceConfig = serde_json::from_value(ds)
        .map_err(|e| ConnectorError::ConnectionFailed(format!("invalid datasource config: {e}")))?;
    config.normalize();
    config.validate()?;

    Ok((config, params))
}

/// 从 context 字段提取可观测字段（tenant/logID/user/bizInstance），用于 tracing span。
pub struct RequestContext {
    pub tenant_key: String,
    pub log_id: String,
    pub base_open_id: String,
    pub biz_instance_id: String,
}

/// 解析 context（best-effort，失败返回全 "?"）。
pub fn parse_context(payload: &Value) -> RequestContext {
    let ctx = payload.get("context").map(safe_json).unwrap_or(Value::Null);
    let s = |v: &Value| v.as_str().unwrap_or("?").to_string();
    RequestContext {
        tenant_key: s(ctx.get("tenantKey").unwrap_or(&Value::Null)),
        log_id: s(ctx
            .get("bitable")
            .and_then(|b| b.get("logID"))
            .unwrap_or(&Value::Null)),
        base_open_id: s(ctx
            .get("scriptArgs")
            .and_then(|a| a.get("baseOpenID"))
            .unwrap_or(&Value::Null)),
        biz_instance_id: s(ctx.get("bizInstanceID").unwrap_or(&Value::Null)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn min_config() -> serde_json::Value {
        json!({"host":"h","username":"u","password":"p","database":"d"})
    }

    #[test]
    fn parses_string_nested_params() {
        let ds = min_config().to_string();
        let params = json!({ "datasourceConfig": ds }).to_string();
        let payload = json!({ "params": params });
        let (cfg, _) = parse_feishu_params(&payload).unwrap();
        assert_eq!(cfg.host, "h");
        assert_eq!(cfg.port, 5432);
        assert_eq!(cfg.mode, "table");
    }

    #[test]
    fn parses_object_params() {
        let payload = json!({ "params": { "datasourceConfig": min_config() } });
        let (cfg, _) = parse_feishu_params(&payload).unwrap();
        assert_eq!(cfg.database, "d");
    }

    #[test]
    fn parses_double_nested() {
        let payload =
            json!({ "params": { "datasourceConfig": { "datasourceConfig": min_config() } } });
        let (cfg, _) = parse_feishu_params(&payload).unwrap();
        assert_eq!(cfg.username, "u");
    }

    #[test]
    fn rejects_injection_in_table_name() {
        let mut c = min_config();
        c["mode"] = json!("table");
        c["table_name"] = json!("users; DROP TABLE x");
        let payload = json!({ "params": { "datasourceConfig": c } });
        assert!(parse_feishu_params(&payload).is_err());
    }
}
