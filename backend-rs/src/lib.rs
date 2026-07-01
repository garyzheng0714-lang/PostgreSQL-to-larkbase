//! FBIF DataBridge —— 飞书多维表格数据同步连接器后端。
//!
//! 无状态、拉取式 HTTP 服务：飞书 Base 主动调用 `/meta.json`、`/api/table_meta`、
//! `/api/records`，本服务连 PostgreSQL 拉数据并按协议返回。架构、选型与正确性
//! 约束见 `docs/designs/rust-backend-rewrite.md`。

pub mod adapter;
pub mod config;
pub mod handlers;
pub mod metadata_cache;
pub mod protocol;
pub mod server;
pub mod signature;
pub mod util;
