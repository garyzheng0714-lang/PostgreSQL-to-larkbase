//! `POST /api/records` —— 返回分页记录（对应 Python `records.py`）。

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::Value;

use crate::adapter::FetchedRow;
use crate::protocol::response::{ok_body, RecordData, RecordsData};
use crate::protocol::ConnectorError;
use crate::server::AppState;
use crate::signature::VerifiedBody;
use crate::util::id_gen::{make_field_id, make_primary_id};
use crate::util::pagination::{decode_page_token, encode_page_token};
use crate::util::params::parse_feishu_params;

/// 协议超时：records 20s。超时返回 HTTP 200 + QUERY_TIMEOUT（1254500）。
const RECORDS_TIMEOUT: Duration = Duration::from_secs(20);

pub async fn records(State(state): State<AppState>, VerifiedBody(body): VerifiedBody) -> Response {
    metrics::counter!("databridge_requests_total", "endpoint" => "records").increment(1);
    let result = match tokio::time::timeout(RECORDS_TIMEOUT, handle(&state, &body)).await {
        Ok(r) => r,
        Err(_) => Err(ConnectorError::QueryTimeout("records deadline exceeded".into())),
    };
    match result {
        Ok(v) => (axum::http::StatusCode::OK, Json(v)).into_response(),
        Err(e) => {
            metrics::counter!("databridge_errors_total", "endpoint" => "records", "code" => e.code().to_string()).increment(1);
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

async fn handle(state: &AppState, body: &[u8]) -> Result<Value, ConnectorError> {
    let payload: Value = serde_json::from_slice(body)
        .map_err(|_| ConnectorError::Unknown("Invalid JSON body".into()))?;
    let (config, params) = parse_feishu_params(&payload)?;

    let adapter = state
        .registry
        .get_default()
        .ok_or_else(|| ConnectorError::Unknown("no adapter registered".into()))?;

    let max_page_size = params
        .get("maxPageSize")
        .and_then(|v| v.as_i64())
        .unwrap_or(1000)
        .clamp(1, 1000); // 最小 1，避免 maxPageSize=0 时 next_offset 不前进导致死循环
    let page_token = params.get("pageToken").and_then(|v| v.as_str()).unwrap_or("");

    let mut offset = 0i64;
    if !page_token.is_empty() {
        offset = decode_page_token(page_token).unwrap_or(0);
    }

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

    let custom_sql = if config.mode == "sql" {
        config.custom_sql.as_deref()
    } else {
        None
    };

    let rows = adapter
        .fetch_records(
            &config,
            offset,
            fetch_limit,
            &config.schema_name,
            config.table_name.as_deref(),
            config.selected_fields.as_deref(),
            custom_sql,
        )
        .await?;

    let mut has_more = rows.len() as i64 > effective;
    let take = (effective as usize).min(rows.len());
    let result_rows = &rows[..take];

    // 列类型与 fieldID
    let mut col_types: HashMap<String, i32> = HashMap::new();
    let mut col_field_ids: HashMap<String, String> = HashMap::new();
    if !result_rows.is_empty() {
        let cols_meta = if let Some(sql) = custom_sql {
            adapter.get_sql_columns(&config, sql).await
        } else {
            adapter
                .list_columns(
                    &config,
                    &config.schema_name,
                    config.table_name.as_deref().unwrap_or(""),
                )
                .await
        };
        match cols_meta {
            Ok(cols) => {
                for c in cols {
                    col_types.insert(c.name.clone(), adapter.map_field_type(&c.data_type));
                    col_field_ids.insert(c.name.clone(), make_field_id(&c.name));
                }
            }
            Err(_) => {
                for (name, _) in &result_rows[0].cells {
                    col_types.insert(name.clone(), 1);
                    col_field_ids.insert(name.clone(), make_field_id(name));
                }
            }
        }
    }

    // 主键列
    let mut pk_columns: Vec<String> = Vec::new();
    if config.mode == "table" {
        if let Some(t) = &config.table_name {
            pk_columns = adapter
                .get_primary_key_columns(&config, &config.schema_name, t)
                .await
                .unwrap_or_default();
        }
    }

    let mut record_list: Vec<RecordData> = Vec::with_capacity(result_rows.len());
    for (idx, row) in result_rows.iter().enumerate() {
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
        record_list.push(RecordData {
            primary_id,
            data,
        });
    }

    let mut next_token = String::new();
    if has_more {
        match offset.checked_add(effective) {
            Some(next_offset) if next_offset < max_row => {
                next_token = encode_page_token(next_offset);
            }
            _ => has_more = false,
        }
    }

    Ok(ok_body(RecordsData {
        next_page_token: next_token,
        has_more,
        records: record_list,
    }))
}
