//! 生成符合 Bitable fieldID/primaryID 约束的安全 ID（对应 Python `id_generator.py`）。

use md5::{Digest, Md5};

/// 由 PG 列名生成 fieldID：`fld_` + md5(列名)[:16]。
///
/// 用 MD5 保证唯一、避开 Bitable 保留字，固定 20 字符（`fld_` + 16 hex），永远安全。
/// 与 Python 完全一致，保证迁移时 fieldID 稳定。
pub fn make_field_id(column_name: &str) -> String {
    let digest = Md5::digest(column_name.as_bytes());
    let hex = hex::encode(digest);
    format!("fld_{}", &hex[..16])
}

/// 由行主键值生成 primaryID：仅保留 `[A-Za-z0-9_]`，其余替换为 `_`，≤100 字符。
/// 全部被替换（空）时回退 `row_<md5[:12]>`。与 Python 一致，保证迁移时记录映射稳定。
pub fn make_primary_id(raw: &str) -> String {
    let sanitized: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    let s = if sanitized.is_empty() {
        let digest = hex::encode(Md5::digest(raw.as_bytes()));
        format!("row_{}", &digest[..12])
    } else {
        sanitized
    };
    s.chars().take(100).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_id_format() {
        let id = make_field_id("user_name");
        assert!(id.starts_with("fld_"));
        assert_eq!(id.len(), 20);
        assert!(id[4..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn field_id_deterministic() {
        assert_eq!(make_field_id("col"), make_field_id("col"));
        assert_ne!(make_field_id("a"), make_field_id("b"));
    }

    #[test]
    fn primary_id_sanitizes() {
        assert_eq!(make_primary_id("abc-123"), "abc_123");
        assert_eq!(make_primary_id("uuid-with-dash"), "uuid_with_dash");
    }

    #[test]
    fn primary_id_empty_fallback() {
        // 非空输入：每字符映射为 '_' 或自身，结果非空，原样保留
        assert_eq!(make_primary_id("---"), "___");
        assert_eq!(make_primary_id("。。"), "__");
        // 仅空输入触发 row_ 回退（与 Python 一致：sanitized 为空时）
        let id = make_primary_id("");
        assert!(id.starts_with("row_"));
        assert_eq!(id.len(), 16); // "row_" + 12 hex
    }

    #[test]
    fn primary_id_max_100() {
        let long = "a".repeat(200);
        assert_eq!(make_primary_id(&long).len(), 100);
    }

    #[test]
    fn primary_id_charset_compliant() {
        for input in ["1", "abc-def", "a b c", "中文键"] {
            let id = make_primary_id(input);
            assert!(id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
        }
    }
}
