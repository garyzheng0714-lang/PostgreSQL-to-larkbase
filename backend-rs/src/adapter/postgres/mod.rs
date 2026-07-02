//! PostgreSQL 数据源适配器实现（对应 Python `adapters/postgres/service.py`）。

pub mod format;
pub mod pool;
pub mod tls;
pub mod type_map;

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use tokio::net::lookup_host;
use tokio_postgres::error::SqlState;
use tokio_postgres::SimpleQueryMessage;

use super::{ColumnInfo, ConnectionResult, DataSourceAdapter, FetchedRow, TableInfo};
use crate::config::Config;
use crate::protocol::request::DatasourceConfig;
use crate::protocol::ConnectorError;
use pool::PoolManager;

/// 双引号包裹标识符并转义内部 `"`（防注入，纵深防御一层）。
fn quote_ident(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn quote_literal(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

const PRIMARY_KEY_COLUMNS_SQL: &str = "SELECT a.attname::text FROM pg_index i \
     JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey) \
     WHERE i.indrelid = $1::regclass AND i.indisprimary \
     ORDER BY array_position(i.indkey, a.attnum)";

fn order_clause(order_fields: Option<&[String]>) -> String {
    let Some(fields) = order_fields else {
        return String::new();
    };
    if fields.is_empty() {
        return String::new();
    }
    let cols = fields
        .iter()
        .map(|c| format!("{} ASC", quote_ident(c)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(" ORDER BY {cols}")
}

fn keyset_where_clause(
    order_fields: Option<&[String]>,
    keyset_after: Option<&[String]>,
) -> Result<String, ConnectorError> {
    let (Some(fields), Some(values)) = (order_fields, keyset_after) else {
        return Ok(String::new());
    };
    if fields.is_empty() || fields.len() != values.len() {
        return Err(ConnectorError::InvalidPageToken);
    }
    if fields.len() == 1 {
        return Ok(format!(
            " WHERE {} > {}",
            quote_ident(&fields[0]),
            quote_literal(&values[0])
        ));
    }
    let cols = fields
        .iter()
        .map(|c| quote_ident(c))
        .collect::<Vec<_>>()
        .join(", ");
    let vals = values
        .iter()
        .map(|v| quote_literal(v))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(" WHERE ({cols}) > ({vals})"))
}

struct FetchQuery<'a> {
    offset: i64,
    limit: i64,
    schema: &'a str,
    table: Option<&'a str>,
    selected_fields: Option<&'a [String]>,
    custom_sql: Option<&'a str>,
    order_fields: Option<&'a [String]>,
    keyset_after: Option<&'a [String]>,
}

fn build_fetch_query(args: FetchQuery<'_>) -> Result<String, ConnectorError> {
    if let Some(sql) = args.custom_sql {
        let sql = sql.trim().trim_end_matches(';').trim_end();
        return Ok(format!(
            "{sql}\nOFFSET {} LIMIT {}",
            args.offset, args.limit
        ));
    }

    let cols = match args.selected_fields {
        Some(f) if !f.is_empty() => f
            .iter()
            .map(|c| quote_ident(c))
            .collect::<Vec<_>>()
            .join(", "),
        _ => "*".to_string(),
    };
    let table = args.table.ok_or(ConnectorError::TableNotFound)?;
    let keyset_where = keyset_where_clause(args.order_fields, args.keyset_after)?;
    let order = order_clause(args.order_fields);
    Ok(format!(
        "SELECT {cols} FROM {}.{}{keyset_where}{order} OFFSET {} LIMIT {}",
        quote_ident(args.schema),
        quote_ident(table),
        args.offset,
        args.limit
    ))
}

fn is_private_or_local_addr(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1])
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || (ip.segments()[0] & 0xfe00) == 0xfc00 // unique local fc00::/7
                || (ip.segments()[0] & 0xffc0) == 0xfe80 // link local fe80::/10
        }
    }
}

async fn host_resolves_to_private_or_local(host: &str, port: u16) -> bool {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return is_private_or_local_addr(&ip);
    }
    match tokio::time::timeout(Duration::from_secs(2), lookup_host((host, port))).await {
        Ok(Ok(mut addrs)) => addrs.any(|addr| is_private_or_local_addr(&addr.ip())),
        _ => false,
    }
}

fn connection_failure_message(detail: &str, private_or_local: bool) -> &'static str {
    if detail.contains("password") || detail.contains("authentication") {
        "用户名或密码错误 / Invalid username or password"
    } else if detail.contains("does not exist") || detail.contains("catalog") {
        "数据库不存在 / Database does not exist"
    } else if private_or_local {
        "当前部署环境无法访问该内网/本地数据库地址，请改用公网地址或打通网络 / This deployment cannot reach the private/local database address; use a public endpoint or network bridge"
    } else {
        "无法连接到服务器，请检查地址和端口 / Cannot connect, check host and port"
    }
}

/// 把 tokio_postgres 错误映射到协议错误。
fn map_pg_err(e: tokio_postgres::Error) -> ConnectorError {
    if let Some(code) = e.code() {
        return match *code {
            SqlState::INVALID_CATALOG_NAME | SqlState::UNDEFINED_TABLE => {
                ConnectorError::TableNotFound
            }
            SqlState::INSUFFICIENT_PRIVILEGE => ConnectorError::PermissionDenied(e.to_string()),
            SqlState::SYNTAX_ERROR | SqlState::UNDEFINED_COLUMN => {
                ConnectorError::InvalidSql(e.to_string())
            }
            SqlState::QUERY_CANCELED => ConnectorError::QueryTimeout(e.to_string()),
            SqlState::INVALID_PASSWORD => ConnectorError::ConnectionFailed(e.to_string()),
            _ => ConnectorError::ConnectionFailed(e.to_string()),
        };
    }
    ConnectorError::ConnectionFailed(e.to_string())
}

/// PostgreSQL 适配器，持有连接池管理器。
pub struct PostgresAdapter {
    pools: Arc<PoolManager>,
}

impl PostgresAdapter {
    pub fn new(cfg: &Config) -> Self {
        Self {
            pools: PoolManager::new(cfg),
        }
    }

    pub fn pools(&self) -> Arc<PoolManager> {
        self.pools.clone()
    }
}

#[async_trait]
impl DataSourceAdapter for PostgresAdapter {
    fn source_type(&self) -> &'static str {
        "postgres"
    }

    async fn test_connection(&self, cfg: &DatasourceConfig) -> ConnectionResult {
        let client = match self.pools.acquire(cfg).await {
            Ok(c) => c,
            Err(e) => {
                let detail = e.to_string().to_lowercase();
                let private_or_local = host_resolves_to_private_or_local(&cfg.host, cfg.port).await;
                let message = connection_failure_message(&detail, private_or_local);
                return ConnectionResult {
                    success: false,
                    message: message.into(),
                    ..Default::default()
                };
            }
        };

        // version() 失败说明连接虽建立但不可用（权限/会话问题），如实报失败。
        let version: String = match client.query_one("SELECT version()", &[]).await {
            Ok(r) => r.get::<_, String>(0),
            Err(_) => {
                return ConnectionResult {
                    success: false,
                    message: "连接已建立但查询失败，请检查权限 / Connected but query failed, check privileges".into(),
                    ..Default::default()
                };
            }
        };
        let short_version = {
            let parts: Vec<&str> = version.split_whitespace().take(2).collect();
            parts.join(" ")
        };
        let database_size: String = client
            .query_one(
                "SELECT pg_size_pretty(pg_database_size(current_database()))",
                &[],
            )
            .await
            .map(|r| r.get::<_, String>(0))
            .unwrap_or_default();
        let table_count: i64 = client
            .query_one(
                "SELECT count(*)::bigint FROM information_schema.tables \
                 WHERE table_schema NOT IN ('pg_catalog','information_schema','pg_toast')",
                &[],
            )
            .await
            .map(|r| r.get::<_, i64>(0))
            .unwrap_or(0);

        ConnectionResult {
            success: true,
            message: String::new(),
            server_version: short_version,
            database_size,
            table_count,
        }
    }

    async fn list_databases(&self, cfg: &DatasourceConfig) -> Result<Vec<String>, ConnectorError> {
        let client = self.pools.acquire(cfg).await?;
        let rows = client
            .query(
                "SELECT datname::text FROM pg_database WHERE datistemplate = false ORDER BY datname",
                &[],
            )
            .await
            .map_err(map_pg_err)?;
        Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
    }

    async fn list_schemas(&self, cfg: &DatasourceConfig) -> Result<Vec<String>, ConnectorError> {
        let client = self.pools.acquire(cfg).await?;
        let rows = client
            .query(
                "SELECT schema_name::text FROM information_schema.schemata \
                 WHERE schema_name NOT IN ('pg_catalog','information_schema','pg_toast') \
                 ORDER BY schema_name",
                &[],
            )
            .await
            .map_err(map_pg_err)?;
        Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
    }

    async fn list_tables(
        &self,
        cfg: &DatasourceConfig,
        schema: &str,
    ) -> Result<Vec<TableInfo>, ConnectorError> {
        let client = self.pools.acquire(cfg).await?;
        let rows = client
            .query(
                "SELECT t.table_name::text, t.table_type::text, \
                 COALESCE(c.reltuples,0)::bigint AS estimated_rows \
                 FROM information_schema.tables t \
                 LEFT JOIN pg_class c ON c.relname = t.table_name \
                 AND c.relnamespace = (SELECT oid FROM pg_namespace WHERE nspname = $1) \
                 WHERE t.table_schema = $1 ORDER BY t.table_name",
                &[&schema],
            )
            .await
            .map_err(map_pg_err)?;
        Ok(rows
            .iter()
            .map(|r| {
                let table_type: String = r.get(1);
                let est: i64 = r.get(2);
                TableInfo {
                    name: r.get::<_, String>(0),
                    kind: if table_type.contains("VIEW") {
                        "view"
                    } else {
                        "table"
                    }
                    .into(),
                    estimated_rows: est.max(0),
                }
            })
            .collect())
    }

    async fn list_columns(
        &self,
        cfg: &DatasourceConfig,
        schema: &str,
        table: &str,
    ) -> Result<Vec<ColumnInfo>, ConnectorError> {
        let client = self.pools.acquire(cfg).await?;
        let rows = client
            .query(
                "SELECT column_name::text, data_type::text, is_nullable::text, ordinal_position::int \
                 FROM information_schema.columns \
                 WHERE table_schema = $1 AND table_name = $2 ORDER BY ordinal_position",
                &[&schema, &table],
            )
            .await
            .map_err(map_pg_err)?;
        Ok(rows
            .iter()
            .map(|r| ColumnInfo {
                name: r.get::<_, String>(0),
                data_type: r.get::<_, String>(1),
                is_nullable: r.get::<_, String>(2) == "YES",
                ordinal_position: r.get::<_, i32>(3),
            })
            .collect())
    }

    async fn get_sql_columns(
        &self,
        cfg: &DatasourceConfig,
        sql: &str,
    ) -> Result<Vec<ColumnInfo>, ConnectorError> {
        let client = self.pools.acquire(cfg).await?;
        let sql = sql.trim_end_matches(';');
        let stmt = client
            .prepare(&format!("SELECT * FROM ({sql}) AS _sub LIMIT 0"))
            .await
            .map_err(map_pg_err)?;
        Ok(stmt
            .columns()
            .iter()
            .enumerate()
            .map(|(i, c)| ColumnInfo {
                name: c.name().to_string(),
                data_type: c.type_().name().to_string(),
                is_nullable: true,
                ordinal_position: (i + 1) as i32,
            })
            .collect())
    }

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
    ) -> Result<Vec<FetchedRow>, ConnectorError> {
        let client = self.pools.acquire(cfg).await?;
        // offset/limit 为本服务校验过的 i64，内联安全（非用户文本）。
        let query = build_fetch_query(FetchQuery {
            offset,
            limit,
            schema,
            table,
            selected_fields,
            custom_sql,
            order_fields,
            keyset_after,
        })?;
        simple_query_rows(&client, &query).await
    }

    async fn get_primary_key_columns(
        &self,
        cfg: &DatasourceConfig,
        schema: &str,
        table: &str,
    ) -> Result<Vec<String>, ConnectorError> {
        let client = self.pools.acquire(cfg).await?;
        // 用引用后的标识符喂 regclass，支持大写/特殊字符表名（否则解析失败 → 主键丢失）。
        let rel = format!("{}.{}", quote_ident(schema), quote_ident(table));
        let rows = client
            .query(PRIMARY_KEY_COLUMNS_SQL, &[&rel])
            .await
            .map_err(map_pg_err)?;
        Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
    }

    async fn preview_sql(
        &self,
        cfg: &DatasourceConfig,
        sql: &str,
        limit: i64,
    ) -> Result<Vec<FetchedRow>, ConnectorError> {
        let client = self.pools.acquire(cfg).await?;
        let sql = sql.trim_end_matches(';');
        let query = format!("SELECT * FROM ({sql}) AS _sub LIMIT {limit}");
        simple_query_rows(&client, &query).await
    }

    async fn validate_sql(
        &self,
        cfg: &DatasourceConfig,
        sql: &str,
    ) -> Result<bool, ConnectorError> {
        let client = self.pools.acquire(cfg).await?;
        let sql = sql.trim_end_matches(';');
        match client.simple_query(&format!("EXPLAIN {sql}")).await {
            Ok(_) => Ok(true),
            Err(e) => Err(ConnectorError::InvalidSql(format!(
                "SQL validation failed: {e}"
            ))),
        }
    }

    fn format_cell(&self, text: Option<&str>, field_type: i32) -> Value {
        format::format_cell(text, field_type)
    }
}

/// 用文本协议执行查询，返回每行的 (列名, 文本值)。
async fn simple_query_rows(
    client: &tokio_postgres::Client,
    query: &str,
) -> Result<Vec<FetchedRow>, ConnectorError> {
    let msgs = client.simple_query(query).await.map_err(map_pg_err)?;
    let mut rows = Vec::new();
    for m in msgs {
        if let SimpleQueryMessage::Row(r) = m {
            let cols = r.columns();
            let mut cells = Vec::with_capacity(cols.len());
            for (i, col) in cols.iter().enumerate() {
                cells.push((col.name().to_string(), r.get(i).map(|s| s.to_string())));
            }
            rows.push(FetchedRow { cells });
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_fetch_query_orders_by_primary_key_when_available() {
        let query = build_fetch_query(FetchQuery {
            offset: 0,
            limit: 1001,
            schema: "public",
            table: Some("articles"),
            selected_fields: None,
            custom_sql: None,
            order_fields: Some(&["id".to_string()]),
            keyset_after: None,
        })
        .unwrap();

        assert_eq!(
            query,
            r#"SELECT * FROM "public"."articles" ORDER BY "id" ASC OFFSET 0 LIMIT 1001"#
        );
    }

    #[test]
    fn table_fetch_query_quotes_selected_columns_and_order_columns() {
        let query = build_fetch_query(FetchQuery {
            offset: 1000,
            limit: 500,
            schema: "tenant schema",
            table: Some("Feed Items"),
            selected_fields: Some(&["Title".to_string(), "created at".to_string()]),
            custom_sql: None,
            order_fields: Some(&["created at".to_string(), "Title".to_string()]),
            keyset_after: None,
        })
        .unwrap();

        assert_eq!(
            query,
            r#"SELECT "Title", "created at" FROM "tenant schema"."Feed Items" ORDER BY "created at" ASC, "Title" ASC OFFSET 1000 LIMIT 500"#
        );
    }

    #[test]
    fn custom_sql_fetch_query_keeps_existing_protocol_shape() {
        let query = build_fetch_query(FetchQuery {
            offset: 10,
            limit: 20,
            schema: "ignored",
            table: None,
            selected_fields: None,
            custom_sql: Some("select id from articles order by id;"),
            order_fields: Some(&["id".to_string()]),
            keyset_after: Some(&["10".to_string()]),
        })
        .unwrap();

        assert_eq!(
            query,
            "select id from articles order by id\nOFFSET 10 LIMIT 20"
        );
    }

    #[test]
    fn custom_sql_fetch_query_line_comment_cannot_swallow_pagination() {
        let query = build_fetch_query(FetchQuery {
            offset: 10,
            limit: 20,
            schema: "ignored",
            table: None,
            selected_fields: None,
            custom_sql: Some("select id from articles order by id -- stable pagination"),
            order_fields: None,
            keyset_after: None,
        })
        .unwrap();

        assert_eq!(
            query,
            "select id from articles order by id -- stable pagination\nOFFSET 10 LIMIT 20"
        );
    }

    #[test]
    fn table_fetch_query_uses_keyset_predicate_when_available() {
        let query = build_fetch_query(FetchQuery {
            offset: 0,
            limit: 100,
            schema: "public",
            table: Some("events"),
            selected_fields: None,
            custom_sql: None,
            order_fields: Some(&["created at".to_string(), "id".to_string()]),
            keyset_after: Some(&["2026-07-01 00:00:00".to_string(), "abc'123".to_string()]),
        })
        .unwrap();

        assert_eq!(
            query,
            r#"SELECT * FROM "public"."events" WHERE ("created at", "id") > ('2026-07-01 00:00:00', 'abc''123') ORDER BY "created at" ASC, "id" ASC OFFSET 0 LIMIT 100"#
        );
    }

    #[test]
    fn table_fetch_query_rejects_keyset_shape_mismatch() {
        let err = build_fetch_query(FetchQuery {
            offset: 0,
            limit: 100,
            schema: "public",
            table: Some("events"),
            selected_fields: None,
            custom_sql: None,
            order_fields: Some(&["id".to_string()]),
            keyset_after: Some(&["1".to_string(), "2".to_string()]),
        })
        .unwrap_err();

        assert!(matches!(err, ConnectorError::InvalidPageToken));
    }

    #[test]
    fn detects_private_and_local_addresses() {
        assert!(is_private_or_local_addr(&"10.80.1.78".parse().unwrap()));
        assert!(is_private_or_local_addr(&"172.16.0.1".parse().unwrap()));
        assert!(is_private_or_local_addr(&"192.168.1.5".parse().unwrap()));
        assert!(is_private_or_local_addr(&"127.0.0.1".parse().unwrap()));
        assert!(!is_private_or_local_addr(
            &"47.102.239.123".parse().unwrap()
        ));
    }

    #[test]
    fn connection_message_prefers_auth_errors_over_private_address_hint() {
        let msg = connection_failure_message("password authentication failed", true);
        assert!(msg.contains("Invalid username or password"));
    }

    #[test]
    fn primary_key_columns_query_preserves_index_order() {
        assert!(PRIMARY_KEY_COLUMNS_SQL.contains("ORDER BY array_position(i.indkey, a.attnum)"));
    }
}
