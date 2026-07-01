//! `POST /api/records` —— 返回分页记录（对应 Python `records.py`）。

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use crate::adapter::{DataSourceAdapter, FetchedRow};
use crate::metadata_cache::{metadata_cache_key, CachedMetadata};
use crate::protocol::response::{ok_body, RecordData, RecordsData};
use crate::protocol::ConnectorError;
use crate::server::AppState;
use crate::signature::VerifiedBody;
use crate::util::id_gen::{make_field_id, make_primary_id};
use crate::util::pagination::{
    decode_keyset_page_token, decode_page_token, encode_keyset_page_token, encode_page_token,
    is_protocol_page_token, KeysetPageToken,
};
use crate::util::params::parse_feishu_params;

/// 协议超时：records 20s。超时返回 HTTP 200 + QUERY_TIMEOUT（1254500）。
const RECORDS_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_PROTOCOL_PAGE_SIZE: i64 = 1000;
const MAX_SAFE_PAGE_SIZE: i64 = 100;
const MAX_RECORDS_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const RESPONSE_WRAPPER_BUDGET: usize = 4096;

enum PageCursor {
    Offset(i64),
    Keyset(KeysetPageToken),
}

pub async fn records(State(state): State<AppState>, VerifiedBody(body): VerifiedBody) -> Response {
    metrics::counter!("databridge_requests_total", "endpoint" => "records").increment(1);
    let started_at = Instant::now();
    let result = match tokio::time::timeout(RECORDS_TIMEOUT, handle(&state, &body)).await {
        Ok(r) => r,
        Err(_) => Err(ConnectorError::QueryTimeout(
            "records deadline exceeded".into(),
        )),
    };
    json_response("records", started_at, result)
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
            metrics::counter!("databridge_errors_total", "endpoint" => "records", "code" => e.code().to_string()).increment(1);
            metrics::histogram!("databridge_request_duration_seconds", "endpoint" => endpoint, "status" => "error")
                .record(started_at.elapsed().as_secs_f64());
            e.into_response()
        }
    }
}

/// 取主键列文本（对应 Python `str(row.get(pk, ""))`）：
/// 列存在且非空 → 值；列存在但 NULL → "None"；列缺失 → ""。
fn pk_part(row: &FetchedRow, pk: &str) -> String {
    match row.cells.iter().find(|(n, _)| n == pk) {
        Some((_, Some(v))) => v.clone(),
        Some((_, None)) => "None".to_string(),
        None => String::new(),
    }
}

fn cap_page_size(requested: i64, _selected_fields: Option<&[String]>) -> i64 {
    let requested = requested.clamp(1, MAX_PROTOCOL_PAGE_SIZE);
    requested.min(MAX_SAFE_PAGE_SIZE)
}

fn selected_fields_include_all_pk(
    selected_fields: Option<&[String]>,
    pk_columns: &[String],
) -> bool {
    let Some(fields) = selected_fields else {
        return true;
    };
    pk_columns.iter().all(|pk| fields.iter().any(|f| f == pk))
}

fn row_pk_values(row: &FetchedRow, pk_columns: &[String]) -> Option<Vec<String>> {
    pk_columns
        .iter()
        .map(|pk| {
            row.cells
                .iter()
                .find(|(name, _)| name == pk)
                .and_then(|(_, value)| value.clone())
        })
        .collect()
}

fn parse_page_cursor(
    page_token: &str,
    keyset_allowed: bool,
    pk_columns: &[String],
) -> Result<Option<PageCursor>, ConnectorError> {
    if page_token.is_empty() {
        return Ok(None);
    }
    if !is_protocol_page_token(page_token) {
        return Err(ConnectorError::InvalidPageToken);
    }
    if page_token.starts_with("k_") {
        if !keyset_allowed {
            return Err(ConnectorError::InvalidPageToken);
        }
        let cursor =
            decode_keyset_page_token(page_token).ok_or(ConnectorError::InvalidPageToken)?;
        if cursor.values.len() != pk_columns.len() {
            return Err(ConnectorError::InvalidPageToken);
        }
        return Ok(Some(PageCursor::Keyset(cursor)));
    }
    let offset = decode_page_token(page_token).map_err(|_| ConnectorError::InvalidPageToken)?;
    Ok(Some(PageCursor::Offset(offset)))
}

fn page_token_param(params: &Value) -> Result<&str, ConnectorError> {
    match params.get("pageToken") {
        Some(Value::String(token)) => Ok(token.as_str()),
        Some(Value::Null) | None => Ok(""),
        Some(_) => Err(ConnectorError::InvalidPageToken),
    }
}

fn ok_records_body_with_limit(
    data: RecordsData,
    max_bytes: usize,
) -> Result<Value, ConnectorError> {
    let body = ok_body(data);
    let body_bytes = serde_json::to_vec(&body)
        .map_err(|e| ConnectorError::Unknown(format!("serialize response: {e}")))?;
    if body_bytes.len() > max_bytes {
        return Err(ConnectorError::ResponseTooLarge);
    }
    Ok(body)
}

fn ok_records_body(data: RecordsData) -> Result<Value, ConnectorError> {
    ok_records_body_with_limit(data, MAX_RECORDS_RESPONSE_BYTES)
}

async fn load_metadata(
    state: &AppState,
    adapter: &Arc<dyn DataSourceAdapter>,
    config: &crate::protocol::request::DatasourceConfig,
    custom_sql: Option<&str>,
) -> Result<CachedMetadata, ConnectorError> {
    let key = metadata_cache_key(config);
    if let Some(cached) = state.metadata_cache.get(&key).await {
        metrics::counter!("databridge_metadata_cache_total", "endpoint" => "records", "result" => "hit")
            .increment(1);
        return Ok(cached);
    }

    metrics::counter!("databridge_metadata_cache_total", "endpoint" => "records", "result" => "miss")
        .increment(1);
    let started_at = Instant::now();
    let columns = if let Some(sql) = custom_sql {
        adapter.get_sql_columns(config, sql).await?
    } else {
        adapter
            .list_columns(
                config,
                &config.schema_name,
                config.table_name.as_deref().unwrap_or(""),
            )
            .await?
    };
    let pk_columns = if custom_sql.is_none() && config.mode == "table" {
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

    metrics::histogram!("databridge_records_stage_seconds", "stage" => "metadata")
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
    let (config, params) = parse_feishu_params(&payload)?;

    let adapter = state
        .registry
        .get_default()
        .ok_or_else(|| ConnectorError::Unknown("no adapter registered".into()))?;

    let requested_page_size = params
        .get("maxPageSize")
        .and_then(|v| v.as_i64())
        .unwrap_or(1000)
        .clamp(1, MAX_PROTOCOL_PAGE_SIZE); // 最小 1，避免 maxPageSize=0 时 next_offset 不前进导致死循环
    let max_page_size = cap_page_size(requested_page_size, config.selected_fields.as_deref());
    let page_token = page_token_param(&params)?;

    let custom_sql = if config.mode == "sql" {
        config.custom_sql.as_deref()
    } else {
        None
    };

    let metadata = load_metadata(state, &adapter, &config, custom_sql).await?;
    let order_fields = if config.mode == "table" && !metadata.pk_columns.is_empty() {
        Some(metadata.pk_columns.as_slice())
    } else {
        None
    };
    let keyset_allowed = order_fields.is_some()
        && selected_fields_include_all_pk(config.selected_fields.as_deref(), &metadata.pk_columns);
    let page_cursor = parse_page_cursor(page_token, keyset_allowed, &metadata.pk_columns)?;
    let keyset_cursor = match &page_cursor {
        Some(PageCursor::Keyset(cursor)) => Some(cursor),
        _ => None,
    };

    let offset = match &page_cursor {
        Some(PageCursor::Keyset(cursor)) => cursor.offset,
        Some(PageCursor::Offset(offset)) => *offset,
        None => 0,
    };

    let max_row = state.cfg.max_row_limit;
    if offset >= max_row {
        return Ok(ok_body(RecordsData {
            next_page_token: String::new(),
            has_more: false,
            records: vec![],
        }));
    }

    let remaining = max_row - offset; // > 0（前面已保证 offset < max_row）
    let effective = max_page_size.min(remaining); // 本页实际行数上限，≥1
    let fetch_limit = effective + 1; // +1 哨兵探测 has_more
    let query_offset = if keyset_cursor.is_some() { 0 } else { offset };
    let keyset_after = keyset_cursor.map(|cursor| cursor.values.as_slice());

    let fetch_started_at = Instant::now();
    let rows = adapter
        .fetch_records(
            &config,
            query_offset,
            fetch_limit,
            &config.schema_name,
            config.table_name.as_deref(),
            config.selected_fields.as_deref(),
            custom_sql,
            order_fields,
            keyset_after,
        )
        .await?;
    metrics::histogram!("databridge_records_stage_seconds", "stage" => "fetch")
        .record(fetch_started_at.elapsed().as_secs_f64());

    let result_rows = rows.iter().take(effective as usize);

    // 列类型与 fieldID
    let mut col_types: HashMap<String, i32> = HashMap::new();
    let mut col_field_ids: HashMap<String, String> = HashMap::new();
    for c in &metadata.columns {
        col_types.insert(c.name.clone(), adapter.map_field_type(&c.data_type));
        col_field_ids.insert(c.name.clone(), make_field_id(&c.name));
    }

    // 主键列
    let pk_columns = metadata.pk_columns;

    let format_started_at = Instant::now();
    let mut record_list: Vec<RecordData> = Vec::with_capacity(result_rows.len());
    let max_records_payload_bytes =
        MAX_RECORDS_RESPONSE_BYTES.saturating_sub(RESPONSE_WRAPPER_BUDGET);
    let mut response_bytes_estimate = RESPONSE_WRAPPER_BUDGET;
    let mut last_pk_values: Option<Vec<String>> = None;
    for (idx, row) in result_rows.enumerate() {
        // 仅当所有主键列都在返回行中时才用主键生成 primaryID；否则（如 selected_fields
        // 排除了主键列）回退到唯一的行号，避免 pk_part 全空串导致 primaryID 重复。
        let pk_present = !pk_columns.is_empty()
            && pk_columns
                .iter()
                .all(|pk| row.cells.iter().any(|(n, _)| n == pk));
        let primary_id = if pk_present {
            let parts: Vec<String> = pk_columns.iter().map(|pk| pk_part(row, pk)).collect();
            make_primary_id(&parts.join("_"))
        } else {
            make_primary_id(&(offset + idx as i64 + 1).to_string())
        };

        let mut data: BTreeMap<String, Value> = BTreeMap::new();
        for (col_name, cell) in &row.cells {
            let field_id = col_field_ids
                .get(col_name)
                .cloned()
                .unwrap_or_else(|| make_field_id(col_name));
            let field_type = col_types.get(col_name).copied().unwrap_or(1);
            data.insert(field_id, adapter.format_cell(cell.as_deref(), field_type));
        }
        let record = RecordData { primary_id, data };
        let record_bytes = serde_json::to_vec(&record).map(|b| b.len()).unwrap_or(0);
        if record_bytes > max_records_payload_bytes {
            return Err(ConnectorError::ResponseTooLarge);
        }
        if !record_list.is_empty()
            && response_bytes_estimate.saturating_add(record_bytes) > max_records_payload_bytes
        {
            break;
        }
        response_bytes_estimate = response_bytes_estimate.saturating_add(record_bytes);
        if keyset_allowed {
            last_pk_values = row_pk_values(row, &pk_columns);
        }
        record_list.push(record);
    }
    metrics::histogram!("databridge_records_stage_seconds", "stage" => "format")
        .record(format_started_at.elapsed().as_secs_f64());
    metrics::histogram!("databridge_records_rows", "endpoint" => "records")
        .record(record_list.len() as f64);

    let mut next_token = String::new();
    let mut has_more = rows.len() as i64 > record_list.len() as i64;
    if has_more {
        match offset.checked_add(record_list.len() as i64) {
            Some(next_offset) if next_offset < max_row => {
                next_token = last_pk_values
                    .as_deref()
                    .and_then(|values| encode_keyset_page_token(next_offset, values))
                    .unwrap_or_else(|| encode_page_token(next_offset));
            }
            _ => has_more = false,
        }
    }

    ok_records_body(RecordsData {
        next_page_token: next_token,
        has_more,
        records: record_list,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_full_field_pages_below_protocol_max() {
        assert_eq!(cap_page_size(1000, None), MAX_SAFE_PAGE_SIZE);
        assert_eq!(cap_page_size(50, None), 50);
    }

    #[test]
    fn caps_selected_field_pages_to_safe_fetch_size() {
        let fields = vec!["id".to_string(), "title".to_string()];
        assert_eq!(cap_page_size(1000, Some(&fields)), MAX_SAFE_PAGE_SIZE);
        assert_eq!(cap_page_size(50, Some(&fields)), 50);
    }

    #[test]
    fn keyset_requires_selected_fields_to_include_primary_key() {
        let pk = vec!["id".to_string()];
        let with_pk = vec!["id".to_string(), "title".to_string()];
        let without_pk = vec!["title".to_string()];

        assert!(selected_fields_include_all_pk(None, &pk));
        assert!(selected_fields_include_all_pk(Some(&with_pk), &pk));
        assert!(!selected_fields_include_all_pk(Some(&without_pk), &pk));
    }

    #[test]
    fn extracts_last_row_primary_key_values_for_keyset_token() {
        let row = FetchedRow {
            cells: vec![
                ("id".to_string(), Some("42".to_string())),
                ("title".to_string(), Some("hello".to_string())),
            ],
        };
        let pk = vec!["id".to_string()];

        assert_eq!(row_pk_values(&row, &pk), Some(vec!["42".to_string()]));
    }

    #[test]
    fn rejects_invalid_external_page_tokens() {
        let pk = vec!["id".to_string()];
        assert!(parse_page_cursor("-1", true, &pk).is_err());
        assert!(parse_page_cursor(&"a".repeat(101), true, &pk).is_err());
        assert!(parse_page_cursor("k_1_3132", false, &pk).is_err());
        assert!(parse_page_cursor("k_1_3132_3334", true, &pk).is_err());
    }

    #[test]
    fn rejects_non_string_page_token_param() {
        assert!(page_token_param(&serde_json::json!({"pageToken": 0})).is_err());
        assert!(page_token_param(&serde_json::json!({"pageToken": false})).is_err());
        assert_eq!(
            page_token_param(&serde_json::json!({"pageToken": null})).unwrap(),
            ""
        );
        assert_eq!(page_token_param(&serde_json::json!({})).unwrap(), "");
    }

    #[test]
    fn parses_keyset_page_token_only_when_primary_key_shape_matches() {
        let pk = vec!["id".to_string()];
        let token = encode_keyset_page_token(100, &["abc".to_string()]).unwrap();
        let cursor = parse_page_cursor(&token, true, &pk).unwrap();

        match cursor {
            Some(PageCursor::Keyset(cursor)) => {
                assert_eq!(cursor.offset, 100);
                assert_eq!(cursor.values, vec!["abc".to_string()]);
            }
            _ => panic!("expected keyset cursor"),
        }
    }

    #[test]
    fn final_records_response_size_includes_protocol_wrapper() {
        let mut data = BTreeMap::new();
        data.insert("fld_test".to_string(), Value::String("x".repeat(200)));
        let response = RecordsData {
            next_page_token: String::new(),
            has_more: false,
            records: vec![RecordData {
                primary_id: "1".to_string(),
                data,
            }],
        };

        assert!(ok_records_body_with_limit(response, 128).is_err());
    }
}
