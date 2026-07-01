//! 前端辅助接口 `/api/helper/*`（对应 Python `helper.py`）。
//!
//! ⚠️ 鉴权 fail-closed（设计 §6.4，收紧 Python 行为）：
//! - dev 模式 → 放行；
//! - 配置了 HELPER_API_KEY → 校验 `X-Helper-Api-Key` 头；
//! - 未配置且非 dev → **一律拒绝**（避免公开探测数据库能力）。

use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::protocol::request::DatasourceConfig;
use crate::server::AppState;

/// 连接信息（各 helper 请求共享，serde flatten）。
#[derive(Debug, Clone, Deserialize)]
pub struct HelperConnection {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    #[serde(default)]
    pub ssl_mode: Option<String>,
    #[serde(default)]
    pub ssl_root_cert: Option<String>,
    #[serde(default)]
    pub ssl_cert: Option<String>,
    #[serde(default)]
    pub ssl_key: Option<String>,
    #[serde(default)]
    pub connect_timeout: Option<u64>,
    #[serde(default)]
    pub query_timeout: Option<u64>,
}

fn default_port() -> u16 {
    5432
}

impl HelperConnection {
    fn to_config(&self) -> DatasourceConfig {
        DatasourceConfig {
            host: self.host.clone(),
            port: self.port,
            username: self.username.clone(),
            password: self.password.clone(),
            database: self.database.clone(),
            ssl_mode: self.ssl_mode.clone().unwrap_or_else(|| "disable".into()),
            ssl_root_cert: self.ssl_root_cert.clone(),
            ssl_cert: self.ssl_cert.clone(),
            ssl_key: self.ssl_key.clone(),
            connect_timeout: self.connect_timeout,
            query_timeout: self.query_timeout,
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct HelperTablesRequest {
    #[serde(flatten)]
    conn: HelperConnection,
    #[serde(default = "default_schema")]
    schema_name: String,
}
#[derive(Debug, Deserialize)]
pub struct HelperColumnsRequest {
    #[serde(flatten)]
    conn: HelperConnection,
    #[serde(default = "default_schema")]
    schema_name: String,
    table_name: String,
}
#[derive(Debug, Deserialize)]
pub struct HelperSqlRequest {
    #[serde(flatten)]
    conn: HelperConnection,
    sql: String,
}
fn default_schema() -> String {
    "public".into()
}

/// 鉴权守卫（fail-closed）。
pub struct HelperAuth;

impl FromRequestParts<AppState> for HelperAuth {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if state.cfg.is_dev_mode() {
            return Ok(HelperAuth);
        }
        let provided = parts
            .headers
            .get("x-helper-api-key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        // fail-closed：未配置 key 一律拒绝
        if state.cfg.helper_api_key.is_empty() || provided != state.cfg.helper_api_key {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Invalid or missing helper API key",
            )
                .into_response());
        }
        Ok(HelperAuth)
    }
}

fn fail(message: &str) -> Json<Value> {
    Json(json!({ "success": false, "message": message, "data": [] }))
}

pub async fn test_connection(
    _auth: HelperAuth,
    State(state): State<AppState>,
    Json(req): Json<HelperConnection>,
) -> Json<Value> {
    let Some(adapter) = state.registry.get_default() else {
        return fail("no adapter");
    };
    let result = adapter.test_connection(&req.to_config()).await;
    Json(json!({
        "success": result.success,
        "message": result.message,
        "server_version": result.server_version,
        "database_size": result.database_size,
        "table_count": result.table_count,
    }))
}

pub async fn list_databases(
    _auth: HelperAuth,
    State(state): State<AppState>,
    Json(req): Json<HelperConnection>,
) -> Json<Value> {
    let Some(adapter) = state.registry.get_default() else {
        return fail("no adapter");
    };
    match adapter.list_databases(&req.to_config()).await {
        Ok(dbs) => Json(json!({ "success": true, "data": dbs })),
        Err(_) => fail("获取数据库列表失败"),
    }
}

pub async fn list_schemas(
    _auth: HelperAuth,
    State(state): State<AppState>,
    Json(req): Json<HelperConnection>,
) -> Json<Value> {
    let Some(adapter) = state.registry.get_default() else {
        return fail("no adapter");
    };
    match adapter.list_schemas(&req.to_config()).await {
        Ok(s) => Json(json!({ "success": true, "data": s })),
        Err(_) => fail("获取 Schema 列表失败"),
    }
}

pub async fn list_tables(
    _auth: HelperAuth,
    State(state): State<AppState>,
    Json(req): Json<HelperTablesRequest>,
) -> Json<Value> {
    let Some(adapter) = state.registry.get_default() else {
        return fail("no adapter");
    };
    match adapter
        .list_tables(&req.conn.to_config(), &req.schema_name)
        .await
    {
        Ok(tables) => {
            let data: Vec<Value> = tables
                .into_iter()
                .map(|t| json!({ "name": t.name, "type": t.kind, "estimated_rows": t.estimated_rows }))
                .collect();
            Json(json!({ "success": true, "data": data }))
        }
        Err(_) => fail("获取表列表失败"),
    }
}

pub async fn list_columns(
    _auth: HelperAuth,
    State(state): State<AppState>,
    Json(req): Json<HelperColumnsRequest>,
) -> Json<Value> {
    let Some(adapter) = state.registry.get_default() else {
        return fail("no adapter");
    };
    match adapter
        .list_columns(&req.conn.to_config(), &req.schema_name, &req.table_name)
        .await
    {
        Ok(cols) => {
            let data: Vec<Value> = cols
                .into_iter()
                .map(|c| {
                    let bt = adapter.map_field_type(&c.data_type);
                    json!({
                        "name": c.name,
                        "data_type": c.data_type,
                        "is_nullable": c.is_nullable,
                        "ordinal_position": c.ordinal_position,
                        "bitable_type": bt,
                    })
                })
                .collect();
            Json(json!({ "success": true, "data": data }))
        }
        Err(_) => fail("获取列信息失败"),
    }
}

pub async fn preview_sql(
    _auth: HelperAuth,
    State(state): State<AppState>,
    Json(req): Json<HelperSqlRequest>,
) -> Json<Value> {
    let Some(adapter) = state.registry.get_default() else {
        return fail("no adapter");
    };
    let cfg = req.conn.to_config();
    // 校验只读
    if crate::protocol::request::has_dangerous_sql(&req.sql) {
        return fail("SQL 含写操作关键字");
    }
    let columns = match adapter.get_sql_columns(&cfg, &req.sql).await {
        Ok(cols) => cols
            .into_iter()
            .map(|c| json!({ "name": c.name, "data_type": c.data_type }))
            .collect::<Vec<_>>(),
        Err(_) => return fail("SQL 预览失败"),
    };
    let rows = match adapter.preview_sql(&cfg, &req.sql, 10).await {
        Ok(rows) => rows,
        Err(_) => return fail("SQL 预览失败"),
    };
    let preview: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let obj: serde_json::Map<String, Value> = r
                .cells
                .into_iter()
                .map(|(k, v)| (k, v.map(Value::String).unwrap_or(Value::Null)))
                .collect();
            Value::Object(obj)
        })
        .collect();
    Json(json!({ "success": true, "data": { "columns": columns, "rows": preview } }))
}
