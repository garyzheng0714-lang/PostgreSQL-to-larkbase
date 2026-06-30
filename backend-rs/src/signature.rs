//! 飞书 Base 请求 SHA-1 验签（对应 Python `signature.py`）。
//!
//! 协议：`sig = hex(SHA1(timestamp + nonce + secretKey + rawBody))`。
//! **是 SHA-1 拼接，不是 HMAC**。
//!
//! 关键正确性约束：
//! - 必须用**原始 body 字节**参与签名，绝不反序列化后重序列化（否则字节不一致
//!   → 签名不匹配，这正是官方 Node demo `JSON.stringify(body)` 的坑）。
//! - 常量时间比较（防时序侧信道），对应 Python `hmac.compare_digest`。
//! - 防重放：时间戳与当前相差 >300s 拒绝。
//! - 空签名放行分支与 Python 一致。

use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::{FromRef, FromRequest, Request};
use sha1::{Digest, Sha1};
use subtle::ConstantTimeEq;

use crate::config::Config;
use crate::protocol::ConnectorError;

/// 签名时间窗口（秒）。
const SIGNATURE_MAX_AGE_SECONDS: i64 = 300;
/// 请求体大小上限（16 MiB），防止超大 body 撑爆内存。
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

const H_TIMESTAMP: &str = "x-base-request-timestamp";
const H_NONCE: &str = "x-base-request-nonce";
const H_SIGNATURE: &str = "x-base-signature";

/// 计算签名：`hex(SHA1(ts + nonce + secret + body))`。
pub fn compute_signature(timestamp: &str, nonce: &str, secret_key: &str, body: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(timestamp.as_bytes());
    hasher.update(nonce.as_bytes());
    hasher.update(secret_key.as_bytes());
    hasher.update(body);
    hex::encode(hasher.finalize())
}

/// 常量时间比较签名是否匹配。
pub fn verify_signature(
    timestamp: &str,
    nonce: &str,
    secret_key: &str,
    body: &[u8],
    signature: &str,
) -> bool {
    let computed = compute_signature(timestamp, nonce, secret_key, body);
    if computed.len() != signature.len() {
        return false;
    }
    computed.as_bytes().ct_eq(signature.as_bytes()).into()
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 校验请求签名。返回 `Ok(())` 表示通过，否则 `SignatureInvalid`。
///
/// 放行策略（比 Python 更严，修复签名旁路）：**仅 dev 模式**允许无签名；
/// 生产（配置了真实 SECRET_KEY）一律要求有效签名——否则攻击者只需省略
/// `X-Base-Signature` 即可绕过验签。
pub fn validate(
    cfg: &Config,
    timestamp: &str,
    nonce: &str,
    signature: &str,
    body: &[u8],
) -> Result<(), ConnectorError> {
    if signature.is_empty() {
        if cfg.is_dev_mode() {
            return Ok(());
        }
        return Err(ConnectorError::SignatureInvalid);
    }

    // 防重放：时间戳可解析且超窗口则拒绝；解析失败则跳过（与 Python 一致）。
    if let Ok(req_time) = timestamp.parse::<i64>() {
        if (now_unix() - req_time).abs() > SIGNATURE_MAX_AGE_SECONDS {
            return Err(ConnectorError::SignatureInvalid);
        }
    }

    if verify_signature(timestamp, nonce, &cfg.secret_key, body, signature) {
        Ok(())
    } else {
        Err(ConnectorError::SignatureInvalid)
    }
}

/// 已验签的原始请求体 extractor。
///
/// 作为 axum `FromRequest` 实现：取原始 `Bytes`（消费 body，必须是 handler 最后
/// 一个参数）→ 验签 → 通过后才把 raw bytes 交给 handler 反序列化。**禁止**在它
/// 之前放任何消费 body 的 extractor（如 `Json<T>`）。
pub struct VerifiedBody(pub Bytes);

impl<S> FromRequest<S> for VerifiedBody
where
    S: Send + Sync,
    Config: FromRef<S>,
{
    type Rejection = ConnectorError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let cfg = Config::from_ref(state);
        let (parts, body) = req.into_parts();

        let h = |k: &str| -> String {
            parts
                .headers
                .get(k)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string()
        };
        let timestamp = h(H_TIMESTAMP);
        let nonce = h(H_NONCE);
        let signature = h(H_SIGNATURE);

        let bytes = axum::body::to_bytes(body, MAX_BODY_BYTES)
            .await
            .map_err(|_| ConnectorError::Unknown("failed to read request body".into()))?;

        validate(&cfg, &timestamp, &nonce, &signature, &bytes)?;
        Ok(VerifiedBody(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev_cfg() -> Config {
        let mut c = Config::from_env();
        c.secret_key = "testBase".into();
        c
    }
    fn prod_cfg() -> Config {
        let mut c = Config::from_env();
        c.secret_key = "real-secret".into();
        c
    }

    #[test]
    fn valid_signature_passes() {
        let ts = now_unix().to_string();
        let body = br#"{"params":"{}"}"#;
        let sig = compute_signature(&ts, "nonce123", "real-secret", body);
        assert!(validate(&prod_cfg(), &ts, "nonce123", &sig, body).is_ok());
    }

    #[test]
    fn wrong_secret_fails() {
        let ts = now_unix().to_string();
        let body = b"hello";
        let sig = compute_signature(&ts, "n", "OTHER-secret", body);
        assert!(validate(&prod_cfg(), &ts, "n", &sig, body).is_err());
    }

    #[test]
    fn tampered_body_fails() {
        let ts = now_unix().to_string();
        let sig = compute_signature(&ts, "n", "real-secret", b"original");
        assert!(validate(&prod_cfg(), &ts, "n", &sig, b"TAMPERED").is_err());
    }

    #[test]
    fn empty_body_with_valid_sig_passes() {
        let ts = now_unix().to_string();
        let sig = compute_signature(&ts, "n", "real-secret", b"");
        assert!(validate(&prod_cfg(), &ts, "n", &sig, b"").is_ok());
    }

    #[test]
    fn expired_timestamp_fails() {
        let old = (now_unix() - 1000).to_string();
        let body = b"x";
        let sig = compute_signature(&old, "n", "real-secret", body);
        assert!(validate(&prod_cfg(), &old, "n", &sig, body).is_err());
    }

    #[test]
    fn no_signature_prod_rejected_even_with_ts_nonce() {
        // 修复签名旁路：生产模式下，即便带 ts+nonce，缺签名也必须拒绝。
        assert!(validate(&prod_cfg(), "123", "n", "", b"body").is_err());
    }

    #[test]
    fn no_signature_dev_mode_passes() {
        assert!(validate(&dev_cfg(), "", "", "", b"body").is_ok());
        assert!(validate(&dev_cfg(), "123", "n", "", b"body").is_ok());
    }

    #[test]
    fn no_signature_prod_no_headers_fails() {
        assert!(validate(&prod_cfg(), "", "", "", b"body").is_err());
    }

    #[test]
    fn raw_body_byte_exact() {
        // 含多余空格 / 乱序的 body：用 raw bytes 验签，无重序列化，逐字节匹配。
        let ts = now_unix().to_string();
        let body = br#"{  "b":2,   "a":1 }"#;
        let sig = compute_signature(&ts, "n", "real-secret", body);
        assert!(validate(&prod_cfg(), &ts, "n", &sig, body).is_ok());
    }
}
