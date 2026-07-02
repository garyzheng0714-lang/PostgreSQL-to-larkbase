//! PostgreSQL 类型 → 飞书 Bitable 字段类型映射（对应 Python `type_mapper.py`）。
//!
//! 覆盖 information_schema 的 `data_type`（如 `character varying`）与 OID 路径的
//! `pg_type.typname`（如 `varchar`）两套别名，保证 table 模式与自定义 SQL 模式
//! 对同一 PG 类型映射一致（设计 §4.5）。

// Bitable 字段类型枚举
pub const FIELD_TEXT: i32 = 1;
pub const FIELD_NUMBER: i32 = 2;
pub const FIELD_SELECT: i32 = 3;
pub const FIELD_MULTI_SELECT: i32 = 4;
pub const FIELD_DATE: i32 = 5;
pub const FIELD_BARCODE: i32 = 6;
pub const FIELD_CHECKBOX: i32 = 7;
pub const FIELD_CURRENCY: i32 = 8;
pub const FIELD_PHONE: i32 = 9;
pub const FIELD_HYPERLINK: i32 = 10;
pub const FIELD_PROGRESS: i32 = 11;
pub const FIELD_RATING: i32 = 12;
pub const FIELD_GEOLOCATION: i32 = 13;

/// PG 类型 → Bitable 字段类型（55 条，与 Python 完全一致）。
fn lookup(base: &str) -> Option<i32> {
    let t = match base {
        // 文本类
        "text" | "varchar" | "character varying" | "char" | "character" | "bpchar" | "name"
        | "uuid" | "citext" | "xml" | "json" | "jsonb" | "bytea" | "inet" | "cidr" | "macaddr"
        | "macaddr8" | "tsvector" | "tsquery" | "interval" | "bit" | "bit varying" | "varbit"
        | "pg_lsn" | "enum" | "user-defined" => FIELD_TEXT,
        // 数字类
        "int2" | "smallint" | "int4" | "integer" | "int" | "int8" | "bigint" | "float4"
        | "real" | "float8" | "double precision" | "numeric" | "decimal" | "serial"
        | "smallserial" | "bigserial" | "oid" => FIELD_NUMBER,
        // 布尔
        "bool" | "boolean" => FIELD_CHECKBOX,
        // 日期
        "date"
        | "timestamp"
        | "timestamp without time zone"
        | "timestamptz"
        | "timestamp with time zone" => FIELD_DATE,
        // 时间（无日期）→ 文本
        "time" | "time without time zone" | "timetz" | "time with time zone" => FIELD_TEXT,
        // 货币
        "money" => FIELD_CURRENCY,
        _ => return None,
    };
    Some(t)
}

/// 映射 PG 类型名到 Bitable 字段类型。大小写不敏感、去空格、剥离 `(...)` 参数；
/// 数组类型 `xxx[]` → 文本；未知类型 → 文本（不报错）。
pub fn map_pg_type(pg_type: &str) -> i32 {
    let normalized = pg_type.trim().to_lowercase();
    let base = normalized.split('(').next().unwrap_or("").trim();
    if base.ends_with("[]") {
        return FIELD_TEXT;
    }
    // OID 路径的数组 typname 形如 `_int4`（前导下划线）
    if base.starts_with('_') {
        return FIELD_TEXT;
    }
    lookup(base).unwrap_or(FIELD_TEXT)
}

/// 该字段类型能否作为主键（索引列）。
pub fn can_be_primary(field_type: i32) -> bool {
    matches!(
        field_type,
        FIELD_TEXT
            | FIELD_NUMBER
            | FIELD_DATE
            | FIELD_HYPERLINK
            | FIELD_PHONE
            | FIELD_BARCODE
            | FIELD_CURRENCY
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_types() {
        for t in [
            "text",
            "varchar",
            "character varying",
            "uuid",
            "json",
            "jsonb",
        ] {
            assert_eq!(map_pg_type(t), FIELD_TEXT, "{t}");
        }
    }

    #[test]
    fn numeric_types() {
        for t in [
            "int4",
            "integer",
            "bigint",
            "numeric",
            "double precision",
            "serial",
        ] {
            assert_eq!(map_pg_type(t), FIELD_NUMBER, "{t}");
        }
    }

    #[test]
    fn bool_date_money() {
        assert_eq!(map_pg_type("boolean"), FIELD_CHECKBOX);
        assert_eq!(map_pg_type("timestamptz"), FIELD_DATE);
        assert_eq!(map_pg_type("timestamp with time zone"), FIELD_DATE);
        assert_eq!(map_pg_type("money"), FIELD_CURRENCY);
    }

    #[test]
    fn time_is_text() {
        assert_eq!(map_pg_type("time"), FIELD_TEXT);
        assert_eq!(map_pg_type("time without time zone"), FIELD_TEXT);
    }

    #[test]
    fn arrays_and_unknown_fallback_text() {
        assert_eq!(map_pg_type("integer[]"), FIELD_TEXT);
        assert_eq!(map_pg_type("_int4"), FIELD_TEXT); // OID 数组 typname
        assert_eq!(map_pg_type("some_unknown_type"), FIELD_TEXT);
    }

    #[test]
    fn case_and_params_insensitive() {
        assert_eq!(map_pg_type("  VARCHAR(255) "), FIELD_TEXT);
        assert_eq!(map_pg_type("NUMERIC(10,2)"), FIELD_NUMBER);
    }

    #[test]
    fn oid_path_alias_matches_information_schema() {
        // varchar (OID typname) ≡ character varying (information_schema)
        assert_eq!(map_pg_type("varchar"), map_pg_type("character varying"));
        assert_eq!(map_pg_type("int4"), map_pg_type("integer"));
        assert_eq!(map_pg_type("bool"), map_pg_type("boolean"));
    }

    #[test]
    fn primary_capability() {
        assert!(can_be_primary(FIELD_TEXT));
        assert!(can_be_primary(FIELD_NUMBER));
        assert!(can_be_primary(FIELD_CURRENCY));
        assert!(!can_be_primary(FIELD_CHECKBOX));
        assert!(!can_be_primary(FIELD_MULTI_SELECT));
    }
}
