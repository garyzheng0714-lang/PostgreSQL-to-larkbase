//! 分页 token 编解码（对应 Python `pagination.py`）。
//!
//! ⚠️ 修复现有 Python 潜在 bug：Python 用 `base64.urlsafe_b64encode`，其字母表含
//! `-`，而协议要求 nextPageToken 只能 `[A-Za-z0-9_]`（≤100）。这里改用十六进制
//! 编码 offset（字符集 `[0-9a-f]`，必然合规）。token 对飞书不透明（只回传不解析），
//! 故编码方式与 Python 不同不影响功能，仅 oracle 对比时豁免 token 字面值。

const KEYSET_PREFIX: &str = "k_";
const MAX_TOKEN_LEN: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeysetPageToken {
    pub offset: i64,
    pub values: Vec<String>,
}

pub fn is_protocol_page_token(token: &str) -> bool {
    token.len() <= MAX_TOKEN_LEN && token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// 把行 offset 编码为合规 page token（十六进制）。
pub fn encode_page_token(offset: i64) -> String {
    format!("{offset:x}")
}

/// 把 page token 解码回 offset。非法 token 返回解析错误，调用方回退 offset=0。
pub fn decode_page_token(token: &str) -> Result<i64, std::num::ParseIntError> {
    i64::from_str_radix(token, 16)
}

pub fn encode_keyset_page_token(offset: i64, values: &[String]) -> Option<String> {
    let mut parts = Vec::with_capacity(values.len() + 1);
    parts.push(format!("{offset:x}"));
    parts.extend(values.iter().map(|value| hex::encode(value.as_bytes())));
    let token = format!("{KEYSET_PREFIX}{}", parts.join("_"));
    (token.len() <= MAX_TOKEN_LEN).then_some(token)
}

pub fn decode_keyset_page_token(token: &str) -> Option<KeysetPageToken> {
    if !is_protocol_page_token(token) {
        return None;
    }
    let payload = token.strip_prefix(KEYSET_PREFIX)?;
    let mut parts = payload.split('_');
    let offset = i64::from_str_radix(parts.next()?, 16).ok()?;
    let values = parts
        .map(|part| {
            let bytes = hex::decode(part).ok()?;
            String::from_utf8(bytes).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then_some(KeysetPageToken { offset, values })
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

    #[test]
    fn protocol_token_validator_rejects_external_invalid_tokens() {
        assert!(is_protocol_page_token("k_1_3132"));
        assert!(!is_protocol_page_token("k-1"));
        assert!(!is_protocol_page_token(&"a".repeat(101)));
    }

    #[test]
    fn keyset_token_round_trips_and_stays_protocol_compliant_for_short_keys() {
        let values = vec!["1000".to_string()];
        let token = encode_keyset_page_token(1000, &values).unwrap();
        let decoded = decode_keyset_page_token(&token).unwrap();

        assert!(token.len() <= 100);
        assert!(token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
        assert_eq!(
            decoded,
            KeysetPageToken {
                offset: 1000,
                values
            }
        );
    }

    #[test]
    fn keyset_token_refuses_overlong_payloads() {
        let values = vec!["x".repeat(120)];
        assert!(encode_keyset_page_token(1, &values).is_none());
    }

    #[test]
    fn keyset_token_supports_uuid_primary_keys_under_protocol_limit() {
        let values = vec!["550e8400-e29b-41d4-a716-446655440000".to_string()];
        let token = encode_keyset_page_token(1000, &values).unwrap();

        assert!(token.len() <= 100);
        assert_eq!(
            decode_keyset_page_token(&token),
            Some(KeysetPageToken {
                offset: 1000,
                values
            })
        );
    }
}
