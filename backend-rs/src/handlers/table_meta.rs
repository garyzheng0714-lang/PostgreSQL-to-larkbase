//! `POST /api/table_meta` —— 返回表结构（对应 Python `table_meta.py`）。

use std::collections::HashSet;
use std::time::Duration;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::Value;

use crate::protocol::response::{ok_body, FieldMeta, FieldProperty, TableMetaData};
use crate::protocol::ConnectorError;
use crate::server::AppState;
use crate::signature::VerifiedBody;
use crate::util::id_gen::make_field_id;
use crate::util::params::parse_feishu_params;

/// 协议超时：table_meta 10s。超时返回 HTTP 200 + QUERY_TIMEOUT（1254500）。
const TABLE_META_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn table_meta(State(state): State<AppState>, VerifiedBody(body): VerifiedBody) -> Response {
    metrics::counter!("databridge_requests_total", "endpoint" => "table_meta").increment(1);
    let result = match tokio::time::timeout(TABLE_META_TIMEOUT, handle(&state, &body)).await {
        Ok(r) => r,
        Err(_) => Err(ConnectorError::QueryTimeout("table_meta deadline exceeded".into())),
    };
    match result {
        Ok(v) => (axum::http::StatusCode::OK, Json(v)).into_response(),
        Err(e) => {
            metrics::counter!("databridge_errors_total", "endpoint" => "table_meta", "code" => e.code().to_string()).increment(1);
            e.into_response()
        }
    }
}

/// 清洗表名：去 `/\?*[]:`，截断 100 字符（与 Python 一致）。
fn clean_table_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | '?' | '*' | '[' | ']' | ':'))
        .collect();
    cleaned.chars().take(100).collect()
}

/// 清洗字段名：去 `[]`（协议禁止），截断 300 字符（协议上限）。
fn clean_field_name(s: &str) -> String {
    let cleaned: String = s.chars().filter(|c| !matches!(c, '[' | ']')).collect();
    cleaned.chars().take(300).collect()
}

async fn handle(state: &AppState, body: &[u8]) -> Result<Value, ConnectorError> {
    let payload: Value = serde_json::from_slice(body)
        .map_err(|_| ConnectorError::Unknown("Invalid JSON body".into()))?;
    let (config, _params) = parse_feishu_params(&payload)?;

    let adapter = state
        .registry
        .get_default()
        .ok_or_else(|| ConnectorError::Unknown("no adapter registered".into()))?;

    // 取列与表名
    let (columns, table_name) = if config.mode == "sql" && config.custom_sql.is_some() {
        let cols = adapter
            .get_sql_columns(&config, config.custom_sql.as_deref().unwrap())
            .await?;
        (cols, "SQL Query Result".to_string())
    } else {
        let cols = adapter
            .list_columns(
                &config,
                &config.schema_name,
                config.table_name.as_deref().unwrap_or(""),
            )
            .await?;
        (cols, config.table_name.clone().unwrap_or_else(|| "Untitled".into()))
    };

    if columns.is_empty() {
        return Err(ConnectorError::TableNotFound);
    }

    // 字段筛选
    let columns = if let Some(sel) = &config.selected_fields {
        let set: HashSet<&String> = sel.iter().collect();
        columns.into_iter().filter(|c| set.contains(&c.name)).collect::<Vec<_>>()
    } else {
        columns
    };

    // 筛选后为空说明 selected_fields 未命中任何实际列 → 配置错误（避免返回空 fields，
    // 也避免与 records 的字段集合不一致）。
    if columns.is_empty() {
        return Err(ConnectorError::ConnectionFailed(
            "selected fields matched no columns".into(),
        ));
    }

    if columns.len() > 299 {
        return Err(ConnectorError::TooManyFields);
    }

    // 主键列（仅 table 模式）
    let renames = config.field_renames.clone().unwrap_or_default();
    let mut pk_columns: Vec<String> = Vec::new();
    if config.mode == "table" {
        if let Some(t) = &config.table_name {
            pk_columns = adapter
                .get_primary_key_columns(&config, &config.schema_name, t)
                .await
                .unwrap_or_default();
        }
    }

    let mut fields: Vec<FieldMeta> = Vec::with_capacity(columns.len());
    let mut primary_set = false;
    for col in &columns {
        let field_type = adapter.map_field_type(&col.data_type);
        let field_id = make_field_id(&col.name);
        let raw_name = renames.get(&col.name).cloned().unwrap_or_else(|| col.name.clone());
        let display_name = clean_field_name(&raw_name);

        let mut is_primary = false;
        if !primary_set {
            if !pk_columns.is_empty() && pk_columns.contains(&col.name) {
                if adapter.can_be_primary(field_type) {
                    is_primary = true;
                    primary_set = true;
                }
            } else if pk_columns.is_empty() && adapter.can_be_primary(field_type) {
                is_primary = true;
                primary_set = true;
            }
        }

        let property = config
            .number_formats
            .as_ref()
            .and_then(|nf| nf.get(&col.name))
            .map(|fmt| FieldProperty {
                formatter: Some(format!("0.{}", "0".repeat(fmt.precision))),
            });

        fields.push(FieldMeta {
            field_id,
            field_name: display_name,
            field_type,
            is_primary,
            description: Some(format!("PostgreSQL: {}", col.data_type)),
            property,
        });
    }

    // 兜底：若仍无主键，选第一个可作主键的字段
    if !primary_set {
        for f in &mut fields {
            if adapter.can_be_primary(f.field_type) {
                f.is_primary = true;
                break;
            }
        }
    }

    let data = TableMetaData {
        table_name: clean_table_name(&table_name),
        fields,
    };
    Ok(ok_body(data))
}
