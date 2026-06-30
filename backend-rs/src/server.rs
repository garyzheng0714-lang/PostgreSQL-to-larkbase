//! axum 路由装配、共享状态与韧性中间件（设计 §4.1/§6）。

use std::sync::Arc;
use std::time::Duration;

use axum::extract::{DefaultBodyLimit, FromRef, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use metrics_exporter_prometheus::PrometheusHandle;
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::adapter::registry::Registry;
use crate::config::Config;
use crate::handlers::{helper, meta, records, table_meta};
use crate::protocol::ConnectorError;

/// 请求体大小上限（16 MiB），与验签 extractor 一致。
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;
/// 整体请求兜底超时（25s，> records 协议 20s）。查询级超时由 PG statement_timeout 保证。
const BACKSTOP_TIMEOUT: Duration = Duration::from_secs(25);

/// 应用共享状态。
#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub registry: Arc<Registry>,
    pub metrics: PrometheusHandle,
}

// 让 Config 可从 AppState 提取（VerifiedBody extractor 需要）。
impl FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Config {
        state.cfg.clone()
    }
}

/// panic → HTTP 200 协议错误体（设计 §4.6/§6.1），不暴露裸 500。
fn panic_to_protocol_error(_err: Box<dyn std::any::Any + Send + 'static>) -> Response {
    tracing::error!("handler panicked, returning protocol error 1254500");
    ConnectorError::Unknown("internal panic".into()).into_response()
}

async fn metrics_handler(State(state): State<AppState>) -> Response {
    (StatusCode::OK, state.metrics.render()).into_response()
}

async fn ready() -> Response {
    (StatusCode::OK, axum::Json(serde_json::json!({ "status": "ready" }))).into_response()
}

/// 构建应用路由。
pub fn build_router(state: AppState) -> Router {
    let middleware = ServiceBuilder::new()
        // 最外层：捕获 panic → 200 协议错误
        .layer(CatchPanicLayer::custom(panic_to_protocol_error))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::new().allow_methods(Any).allow_headers(Any).allow_origin(Any))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        // 兜底超时返回 504（罕见的整体挂起）；查询级超时由 PG statement_timeout
        // 经 QUERY_CANCELED → QueryTimeout(200) 正常协议体返回。
        .layer(TimeoutLayer::with_status_code(
            StatusCode::GATEWAY_TIMEOUT,
            BACKSTOP_TIMEOUT,
        ));

    Router::new()
        .route("/meta.json", get(meta::get_meta))
        .route("/health", get(meta::health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics_handler))
        .route("/api/table_meta", post(table_meta::table_meta))
        .route("/api/records", post(records::records))
        // 前端辅助接口
        .route("/api/helper/test_connection", post(helper::test_connection))
        .route("/api/helper/databases", post(helper::list_databases))
        .route("/api/helper/schemas", post(helper::list_schemas))
        .route("/api/helper/tables", post(helper::list_tables))
        .route("/api/helper/columns", post(helper::list_columns))
        .route("/api/helper/preview_sql", post(helper::preview_sql))
        .layer(middleware)
        .with_state(state)
}
