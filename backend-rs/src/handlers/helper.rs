//! 前端辅助接口 `/api/helper/*`（对应 Python `helper.py`）。
//!
//! ⚠️ 鉴权 fail-closed（设计 §6.4，收紧 Python 行为）：
//! - dev 模式 → 放行；
//! - 配置了 HELPER_API_KEY → 校验 `X-Helper-Api-Key` 头；
//! - 未配置且非 dev → **一律拒绝**（避免公开探测数据库能力）。
//!
//! key 由 FBIF 中台下发到后端环境，配置页运行时经 `GET /api/helper/key` 来要，不在构建时写进前端。

use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::{header, StatusCode};
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

/// `GET /api/helper/key` —— 把当前 helper key 交给配置页。
/// 这把 key 本来就会到每个打开配置页的浏览器里，只挡泛扫，所以不鉴权；
/// 改成运行时来要，中台换 key 时重建后端就行，前端不用重新构建，两边也不会对不上。
pub async fn key(State(state): State<AppState>) -> Response {
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({ "key": state.cfg.helper_api_key })),
    )
        .into_response()
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use metrics_exporter_prometheus::PrometheusBuilder;
    use tower::ServiceExt;

    use super::*;
    use crate::adapter::registry::Registry;
    use crate::config::Config;
    use crate::metadata_cache::MetadataCache;
    use crate::server::build_router;

    fn app(helper_api_key: &str) -> axum::Router {
        let mut cfg = Config::from_env();
        cfg.secret_key = "real-secret".into();
        cfg.helper_api_key = helper_api_key.into();
        build_router(AppState {
            cfg,
            registry: Arc::new(Registry::new()),
            metrics: PrometheusBuilder::new().build_recorder().handle(),
            metadata_cache: Arc::new(MetadataCache::new(Duration::from_secs(1), 1)),
        })
    }

    fn probe(key: &str) -> Request<Body> {
        Request::post("/api/helper/test_connection")
            .header("content-type", "application/json")
            .header("x-helper-api-key", key)
            .body(Body::from("{}"))
            .unwrap()
    }

    #[tokio::test]
    async fn config_page_gets_the_key_the_guard_accepts() {
        let app = app("hub-key");
        let res = app
            .clone()
            .oneshot(Request::get("/api/helper/key").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[header::CACHE_CONTROL], "no-store");
        let body: Value =
            serde_json::from_slice(&res.into_body().collect().await.unwrap().to_bytes()).unwrap();
        let key = body["key"].as_str().unwrap();
        assert_eq!(key, "hub-key");

        // 鉴权在解析请求体之前：key 对了只会因缺字段被拒，不会是 401
        let ok = app.clone().oneshot(probe(key)).await.unwrap();
        assert_ne!(ok.status(), StatusCode::UNAUTHORIZED);
        let stale = app.oneshot(probe("old-key")).await.unwrap();
        assert_eq!(stale.status(), StatusCode::UNAUTHORIZED);
    }
}
