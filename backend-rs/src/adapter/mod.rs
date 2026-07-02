//! 数据源适配器抽象层（对应 Python `adapters/base.py` + `registry.py`）。
//!
//! `DataSourceAdapter` trait 定义所有数据源必须实现的能力；`registry` 提供
//! 按类型查找。PostgreSQL 为首个、当前唯一实现。
//!
//! 值解码策略：数据行用 PG 文本协议（`simple_query`）取得，每个单元格是
//! `Option<String>`（PG 文本表示，NULL 为 None）。列类型从元数据查询单独获取，
//! 由 handler 计算字段类型后调用 `format_cell` 解析文本 → Bitable 值。这样天然
//! 覆盖所有 PG 类型（含 uuid/json/enum/数组/inet 等），无需逐类型 FromSql。

pub mod postgres;
pub mod registry;

use async_trait::async_trait;
use serde_json::Value;

use crate::protocol::request::DatasourceConfig;
use crate::protocol::ConnectorError;

use postgres::type_map;

/// 列元信息。
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub ordinal_position: i32,
}

/// 表/视图元信息。
#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub kind: String, // "table" | "view"
    pub estimated_rows: i64,
}

/// 连接测试结果。
#[derive(Debug, Clone, Default)]
pub struct ConnectionResult {
    pub success: bool,
    pub message: String,
    pub server_version: String,
    pub database_size: String,
    pub table_count: i64,
}

/// 一行数据：列名 → PG 文本值（NULL 为 None），保留列顺序。
#[derive(Debug, Clone)]
pub struct FetchedRow {
    pub cells: Vec<(String, Option<String>)>,
}

impl FetchedRow {
    /// 按列名取文本值。
    pub fn get(&self, col: &str) -> Option<&str> {
        self.cells
            .iter()
            .find(|(name, _)| name == col)
            .and_then(|(_, v)| v.as_deref())
    }
}

/// 数据源适配器：所有数据源必须实现的能力（对应 Python Protocol，13 方法）。
#[async_trait]
pub trait DataSourceAdapter: Send + Sync {
    fn source_type(&self) -> &'static str;

    async fn test_connection(&self, cfg: &DatasourceConfig) -> ConnectionResult;
    async fn list_databases(&self, cfg: &DatasourceConfig) -> Result<Vec<String>, ConnectorError>;
    async fn list_schemas(&self, cfg: &DatasourceConfig) -> Result<Vec<String>, ConnectorError>;
    async fn list_tables(
        &self,
        cfg: &DatasourceConfig,
        schema: &str,
    ) -> Result<Vec<TableInfo>, ConnectorError>;
    async fn list_columns(
        &self,
        cfg: &DatasourceConfig,
        schema: &str,
        table: &str,
    ) -> Result<Vec<ColumnInfo>, ConnectorError>;
    async fn get_sql_columns(
        &self,
        cfg: &DatasourceConfig,
        sql: &str,
    ) -> Result<Vec<ColumnInfo>, ConnectorError>;
    #[allow(clippy::too_many_arguments)]
    async fn fetch_records(
        &self,
        cfg: &DatasourceConfig,
        offset: i64,
        limit: i64,
        schema: &str,
        table: Option<&str>,
        selected_fields: Option<&[String]>,
        custom_sql: Option<&str>,
        order_fields: Option<&[String]>,
        keyset_after: Option<&[String]>,
    ) -> Result<Vec<FetchedRow>, ConnectorError>;
    async fn get_primary_key_columns(
        &self,
        cfg: &DatasourceConfig,
        schema: &str,
        table: &str,
    ) -> Result<Vec<String>, ConnectorError>;
    async fn preview_sql(
        &self,
        cfg: &DatasourceConfig,
        sql: &str,
        limit: i64,
    ) -> Result<Vec<FetchedRow>, ConnectorError>;
    async fn validate_sql(&self, cfg: &DatasourceConfig, sql: &str)
        -> Result<bool, ConnectorError>;

    /// PG 文本值 → Bitable 协议值（按字段类型）。
    fn format_cell(&self, text: Option<&str>, field_type: i32) -> Value;

    fn map_field_type(&self, pg_type: &str) -> i32 {
        type_map::map_pg_type(pg_type)
    }
    fn can_be_primary(&self, field_type: i32) -> bool {
        type_map::can_be_primary(field_type)
    }
}
