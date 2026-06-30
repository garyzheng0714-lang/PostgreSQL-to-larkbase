//! 用户同步配置 `DatasourceConfig`（对应 Python `datasource.py`）。
//!
//! 飞书在每次 table_meta/records 请求里透传该配置（前端 saveConfigAndGoNext
//! 保存的内容）。含标识符白名单与危险 SQL 黑名单校验。

use serde::Deserialize;

use crate::protocol::ConnectorError;

fn default_port() -> u16 {
    5432
}
fn default_mode() -> String {
    "table".into()
}
fn default_schema() -> String {
    "public".into()
}
fn default_ssl_mode() -> String {
    "disable".into()
}
fn default_auto_sync() -> bool {
    true
}

/// 数字字段格式配置。
#[derive(Debug, Clone, Deserialize, Default)]
pub struct NumberFormat {
    #[serde(default)]
    pub precision: usize,
}

/// 用户保存的数据源同步配置。
#[derive(Debug, Clone, Deserialize)]
pub struct DatasourceConfig {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_schema")]
    pub schema_name: String,
    #[serde(default)]
    pub table_name: Option<String>,
    #[serde(default)]
    pub selected_fields: Option<Vec<String>>,
    #[serde(default)]
    pub custom_sql: Option<String>,
    #[serde(default)]
    pub field_renames: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub number_formats: Option<std::collections::HashMap<String, NumberFormat>>,
    #[serde(default = "default_auto_sync")]
    pub auto_sync: bool,
    #[serde(default = "default_ssl_mode")]
    pub ssl_mode: String,
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

impl Default for DatasourceConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: default_port(),
            username: String::new(),
            password: String::new(),
            database: String::new(),
            mode: default_mode(),
            schema_name: default_schema(),
            table_name: None,
            selected_fields: None,
            custom_sql: None,
            field_renames: None,
            number_formats: None,
            auto_sync: default_auto_sync(),
            ssl_mode: default_ssl_mode(),
            ssl_root_cert: None,
            ssl_cert: None,
            ssl_key: None,
            connect_timeout: None,
            query_timeout: None,
        }
    }
}

const DANGEROUS_KEYWORDS: [&str; 10] = [
    "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "CREATE", "TRUNCATE", "GRANT", "REVOKE", "EXEC",
];

const VALID_SSL_MODES: [&str; 6] = [
    "disable",
    "allow",
    "prefer",
    "require",
    "verify-ca",
    "verify-full",
];

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// 标识符白名单：`^[\w][\w ]*$`（首字符为单词字符，其余单词字符或空格）。
/// 拒绝引号、分号、括号、反斜杠等注入字符。
pub fn valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) if is_word_char(first) => {}
        _ => return false,
    }
    chars.all(|c| is_word_char(c) || c == ' ')
}

/// 危险 SQL 关键字（写操作）整词检测，大小写不敏感。
pub fn has_dangerous_sql(sql: &str) -> bool {
    let up = sql.to_uppercase();
    let bytes = up.as_bytes();
    for kw in DANGEROUS_KEYWORDS {
        let mut start = 0;
        while let Some(pos) = up[start..].find(kw) {
            let abs = start + pos;
            let before_ok = abs == 0 || !bytes[abs - 1].is_ascii_alphanumeric() && bytes[abs - 1] != b'_';
            let after = abs + kw.len();
            let after_ok =
                after >= bytes.len() || !bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_';
            if before_ok && after_ok {
                return true;
            }
            start = abs + kw.len();
        }
    }
    false
}

impl DatasourceConfig {
    /// 校验配置合法性（标识符白名单 + 危险 SQL 黑名单 + 枚举值）。
    pub fn validate(&self) -> Result<(), ConnectorError> {
        if self.mode != "table" && self.mode != "sql" {
            return Err(ConnectorError::ConnectionFailed(format!("invalid mode: {}", self.mode)));
        }
        if !VALID_SSL_MODES.contains(&self.ssl_mode.as_str()) {
            return Err(ConnectorError::ConnectionFailed(format!(
                "invalid ssl_mode: {}",
                self.ssl_mode
            )));
        }
        if !valid_identifier(&self.schema_name) {
            return Err(ConnectorError::ConnectionFailed(format!(
                "invalid schema name: {}",
                self.schema_name
            )));
        }
        if let Some(t) = &self.table_name {
            if !t.is_empty() && !valid_identifier(t) {
                return Err(ConnectorError::ConnectionFailed(format!("invalid table name: {t}")));
            }
        }
        if let Some(fields) = &self.selected_fields {
            for f in fields {
                if !valid_identifier(f) {
                    return Err(ConnectorError::ConnectionFailed(format!("invalid field name: {f}")));
                }
            }
        }
        if let Some(sql) = &self.custom_sql {
            // 拒绝内部分号：custom_sql 经 `SELECT * FROM (sql) AS _sub` 包裹后用
            // simple_query 执行，simple_query 允许多语句；一个顶层 `;` 即可逃逸
            // （`...) AS _sub; SET ...; ...`）污染只读会话。仅允许尾部一个 `;`。
            let trimmed = sql.trim().trim_end_matches(';');
            if trimmed.contains(';') {
                return Err(ConnectorError::InvalidSql(
                    "custom SQL must be a single statement (no ';')".into(),
                ));
            }
            if has_dangerous_sql(sql) {
                return Err(ConnectorError::InvalidSql(
                    "SQL contains disallowed write keywords".into(),
                ));
            }
        }
        Ok(())
    }

    /// 归一化：空 `selected_fields`（`[]`）视为 None（= 全部列），避免 table_meta
    /// 返回空字段而 records 当作 `*` 的不一致。
    pub fn normalize(&mut self) {
        if let Some(v) = &self.selected_fields {
            if v.is_empty() {
                self.selected_fields = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_accepts_normal() {
        assert!(valid_identifier("users"));
        assert!(valid_identifier("my_table"));
        assert!(valid_identifier("订单表"));
        assert!(valid_identifier("col 1"));
    }

    #[test]
    fn identifier_rejects_injection() {
        assert!(!valid_identifier("users; DROP TABLE x"));
        assert!(!valid_identifier("a\"b"));
        assert!(!valid_identifier("a)b"));
        assert!(!valid_identifier(" leading"));
        assert!(!valid_identifier(""));
    }

    #[test]
    fn dangerous_sql_detected() {
        assert!(has_dangerous_sql("INSERT INTO x VALUES (1)"));
        assert!(has_dangerous_sql("select 1; drop table y"));
        assert!(has_dangerous_sql("DELETE FROM t"));
    }

    #[test]
    fn safe_sql_passes() {
        assert!(!has_dangerous_sql("SELECT * FROM users WHERE id = 1"));
        // 含关键字子串但非整词，不应误报
        assert!(!has_dangerous_sql("SELECT created_at, updated_count FROM t"));
    }

    fn sql_config(sql: &str) -> DatasourceConfig {
        DatasourceConfig {
            host: "h".into(),
            username: "u".into(),
            password: "p".into(),
            database: "d".into(),
            mode: "sql".into(),
            custom_sql: Some(sql.into()),
            ..Default::default()
        }
    }

    #[test]
    fn rejects_multi_statement_sql() {
        // 多语句逃逸尝试
        assert!(sql_config("SELECT 1) AS _sub; SET x TO y; SELECT 1").validate().is_err());
        assert!(sql_config("SELECT 1; SELECT 2").validate().is_err());
        // 尾部分号允许
        assert!(sql_config("SELECT 1;").validate().is_ok());
        assert!(sql_config("SELECT * FROM t").validate().is_ok());
    }

    #[test]
    fn normalize_empties_selected_fields() {
        let mut c = DatasourceConfig {
            host: "h".into(),
            username: "u".into(),
            password: "p".into(),
            database: "d".into(),
            selected_fields: Some(vec![]),
            ..Default::default()
        };
        c.normalize();
        assert!(c.selected_fields.is_none());
    }
}
