//! `GET /meta.json` —— 飞书插件元信息（对应 Python `meta.py`）。
//!
//! 关键：`dataSourceConfigUiUri` 必须带动态参数 `?v=<时间戳>`，否则飞书 CDN
//! 会缓存前端页面，更新前端后飞书仍加载旧版（见 CLAUDE.md）。

use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::server::AppState;

/// 插件版本号（更新后需在飞书连接器中心「更新版本」）。
pub const APP_VERSION: &str = "1.3.0";
const INIT_WIDTH: u16 = 520;
const INIT_HEIGHT: u16 = 520;

pub async fn get_meta(State(state): State<AppState>) -> Json<Value> {
    let cfg = &state.cfg;
    let cache_bust = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Json(json!({
        "schemaVersion": 1,
        "version": APP_VERSION,
        "type": "data_connector",
        "extraData": {
            "disabledPeriodicSync": false,
            "dataSourceConfigUiUri": format!("{}?v={}", cfg.frontend_url, cache_bust),
            "initHeight": INIT_HEIGHT,
            "initWidth": INIT_WIDTH,
        },
        "protocol": {
            "type": "http",
            "httpProtocol": {
                "uris": [
                    { "type": "tableMeta", "uri": "/api/table_meta" },
                    { "type": "records", "uri": "/api/records" },
                ]
            },
        },
    }))
}

/// `GET /health` —— 存活探测。
pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
