//! `POST /api/table_meta` —— 返回表结构（对应 Python `table_meta.py`）。

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use crate::adapter::DataSourceAdapter;
use crate::metadata_cache::{CachedMetadata, metadata_cache_key};
use crate::protocol::ConnectorError;
use crate::protocol::response::{FieldMeta, FieldProperty, TableMetaData, ok_body};
use crate::server::AppState;
use crate::signature::VerifiedBody;
use crate::util::id_gen::make_field_id;
use crate::util::params::parse_feishu_params;

/// 协议超时：table_meta 10s。超时返回 HTTP 200 + QUERY_TIMEOUT（1254500）。
const TABLE_META_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn table_meta(
    State(state): State<AppState>,
    VerifiedBody(body): VerifiedBody,
) -> Response {
    metrics::counter!("databridge_requests_total", "endpoint" => "table_meta").increment(1);
    let started_at = Instant::now();
    let result = match tokio::time::timeout(TABLE_META_TIMEOUT, handle(&state, &body)).await {
        Ok(r) => r,
        Err(_) => Err(ConnectorError::QueryTimeout(
            "table_meta deadline exceeded".into(),
        )),
    };
    json_response("table_meta", started_at, result)
}

fn json_response(
    endpoint: &'static str,
    started_at: Instant,
    result: Result<Value, ConnectorError>,
) -> Response {
    match result {
        Ok(v) => match serde_json::to_vec(&v) {
            Ok(bytes) => {
                metrics::histogram!("databridge_request_duration_seconds", "endpoint" => endpoint, "status" => "ok")
                    .record(started_at.elapsed().as_secs_f64());
                metrics::histogram!("databridge_response_bytes", "endpoint" => endpoint)
                    .record(bytes.len() as f64);
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
                    bytes,
                )
                    .into_response()
            }
            Err(e) => ConnectorError::Unknown(format!("serialize response: {e}")).into_response(),
        },
        Err(e) => {
            metrics::counter!("databridge_errors_total", "endpoint" => "table_meta", "code" => e.code().to_string()).increment(1);
            metrics::histogram!("databridge_request_duration_seconds", "endpoint" => endpoint, "status" => "error")
                .record(started_at.elapsed().as_secs_f64());
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

async fn load_metadata(
    state: &AppState,
    adapter: &Arc<dyn DataSourceAdapter>,
    config: &crate::protocol::request::DatasourceConfig,
) -> Result<CachedMetadata, ConnectorError> {
    let key = metadata_cache_key(config);
    if let Some(cached) = state.metadata_cache.get(&key).await {
        metrics::counter!("databridge_metadata_cache_total", "endpoint" => "table_meta", "result" => "hit")
            .increment(1);
        return Ok(cached);
    }

    metrics::counter!("databridge_metadata_cache_total", "endpoint" => "table_meta", "result" => "miss")
        .increment(1);
    let started_at = Instant::now();
    let columns = if config.mode == "sql" && config.custom_sql.is_some() {
        adapter
            .get_sql_columns(config, config.custom_sql.as_deref().unwrap())
            .await?
    } else {
        adapter
            .list_columns(
                config,
                &config.schema_name,
                config.table_name.as_deref().unwrap_or(""),
            )
            .await?
    };
    let pk_columns = if config.mode == "table" {
        if let Some(t) = &config.table_name {
            adapter
                .get_primary_key_columns(config, &config.schema_name, t)
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    metrics::histogram!("databridge_table_meta_stage_seconds", "stage" => "metadata")
        .record(started_at.elapsed().as_secs_f64());

    let metadata = CachedMetadata {
        columns,
        pk_columns,
    };
    state.metadata_cache.insert(key, metadata.clone()).await;
    Ok(metadata)
}

async fn handle(state: &AppState, body: &[u8]) -> Result<Value, ConnectorError> {
    let payload: Value = serde_json::from_slice(body)
        .map_err(|_| ConnectorError::Unknown("Invalid JSON body".into()))?;
    let (config, _params) = parse_feishu_params(&payload)?;

    let adapter = state
        .registry
        .get_default()
        .ok_or_else(|| ConnectorError::Unknown("no adapter registered".into()))?;

    let metadata = load_metadata(state, &adapter, &config).await?;
    let table_name = if config.mode == "sql" && config.custom_sql.is_some() {
        "SQL Query Result".to_string()
    } else {
        config
            .table_name
            .clone()
            .unwrap_or_else(|| "Untitled".into())
    };
    let columns = metadata.columns;

    if columns.is_empty() {
        return Err(ConnectorError::TableNotFound);
    }

    // 字段筛选
    let columns = if let Some(sel) = &config.selected_fields {
        let set: HashSet<&String> = sel.iter().collect();
        columns
            .into_iter()
            .filter(|c| set.contains(&c.name))
            .collect::<Vec<_>>()
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
    let pk_columns = metadata.pk_columns;

    let mut fields: Vec<FieldMeta> = Vec::with_capacity(columns.len());
    let mut primary_set = false;
    for col in &columns {
        let field_type = adapter.map_field_type(&col.data_type);
        let field_id = make_field_id(&col.name);
        let raw_name = renames
            .get(&col.name)
            .cloned()
            .unwrap_or_else(|| col.name.clone());
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
