//! 分页 token 编解码（对应 Python `pagination.py`）。
//!
//! ⚠️ 修复现有 Python 潜在 bug：Python 用 `base64.urlsafe_b64encode`，其字母表含
//! `-`，而协议要求 nextPageToken 只能 `[A-Za-z0-9_]`（≤100）。这里改用十六进制
//! 编码 offset（字符集 `[0-9a-f]`，必然合规）。token 对飞书不透明（只回传不解析），
//! 故编码方式与 Python 不同不影响功能，仅 oracle 对比时豁免 token 字面值。

/// 把行 offset 编码为合规 page token（十六进制）。
pub fn encode_page_token(offset: i64) -> String {
    format!("{offset:x}")
}

/// 把 page token 解码回 offset。非法 token 返回 `Err(())`，调用方回退 offset=0。
pub fn decode_page_token(token: &str) -> Result<i64, ()> {
    i64::from_str_radix(token, 16).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for off in [0i64, 1, 999, 1000, 49999, 50000, i64::MAX] {
            assert_eq!(decode_page_token(&encode_page_token(off)), Ok(off));
        }
    }

    #[test]
    fn token_charset_compliant() {
        // 协议要求 nextPageToken 仅 [A-Za-z0-9_] 且 ≤100
        for off in [0i64, 1, 12345, 50000, i64::MAX] {
            let t = encode_page_token(off);
            assert!(t.len() <= 100);
            assert!(t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
        }
    }

    #[test]
    fn invalid_token_errs() {
        assert!(decode_page_token("not-hex!").is_err());
        assert!(decode_page_token("").is_err());
    }
}
