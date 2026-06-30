//! 按 ssl_mode 构建 PostgreSQL 的 rustls TLS 连接器（对应 Python `ssl_context.py`）。
//!
//! 六种 ssl_mode（设计 §2.3）：
//! - `disable`：不加密（由 tokio_postgres Config.ssl_mode=Disable 控制，此处连接器不被使用）。
//! - `allow`/`prefer`/`require`：加密但不校验证书（accept-any verifier）。
//! - `verify-ca`/`verify-full`：校验证书链（webpki roots + 可选自定义 CA）。
//!   注：当前 verify-ca 同样校验主机名（偏严，errs safe），精确「跳过主机名」待补。

use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, Error as RustlsError, RootCertStore, SignatureScheme};
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::protocol::request::DatasourceConfig;
use crate::protocol::ConnectorError;

/// 不校验证书的 verifier（仅加密，对应 sslmode=require）。
#[derive(Debug)]
struct NoVerify {
    schemes: Vec<SignatureScheme>,
}

impl ServerCertVerifier for NoVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes.clone()
    }
}

/// 构建 rustls TLS 连接器。是否真正握手由 tokio_postgres Config.ssl_mode 决定。
/// CA / 客户端证书解析失败一律报 ConnectionFailed，不静默吞掉。
pub fn build_tls(cfg: &DatasourceConfig) -> Result<MakeRustlsConnect, ConnectorError> {
    let provider = rustls::crypto::ring::default_provider();
    let schemes = provider.signature_verification_algorithms.supported_schemes();
    let provider = Arc::new(provider);

    let verify = matches!(cfg.ssl_mode.as_str(), "verify-ca" | "verify-full");

    // 客户端证书（mTLS），ssl_cert + ssl_key 同时提供时启用。
    let client_auth = match (&cfg.ssl_cert, &cfg.ssl_key) {
        (Some(cert_pem), Some(key_pem)) => {
            let certs = rustls_pemfile::certs(&mut cert_pem.as_bytes())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| ConnectorError::ConnectionFailed(format!("invalid ssl_cert: {e}")))?;
            if certs.is_empty() {
                return Err(ConnectorError::ConnectionFailed("ssl_cert has no certificate".into()));
            }
            let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())
                .map_err(|e| ConnectorError::ConnectionFailed(format!("invalid ssl_key: {e}")))?
                .ok_or_else(|| ConnectorError::ConnectionFailed("ssl_key has no private key".into()))?;
            Some((certs, key))
        }
        _ => None,
    };

    let base = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| ConnectorError::ConnectionFailed(format!("tls config: {e}")))?;

    let with_verifier = if verify {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        if let Some(ca_pem) = &cfg.ssl_root_cert {
            let cas = rustls_pemfile::certs(&mut ca_pem.as_bytes())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| ConnectorError::ConnectionFailed(format!("invalid ssl_root_cert: {e}")))?;
            if cas.is_empty() {
                return Err(ConnectorError::ConnectionFailed("ssl_root_cert has no certificate".into()));
            }
            for c in cas {
                roots
                    .add(c)
                    .map_err(|e| ConnectorError::ConnectionFailed(format!("add ca: {e}")))?;
            }
        }
        base.with_root_certificates(roots)
    } else {
        base.dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerify { schemes }))
    };

    let client_config = match client_auth {
        Some((certs, key)) => with_verifier
            .with_client_auth_cert(certs, key)
            .map_err(|e| ConnectorError::ConnectionFailed(format!("client auth cert: {e}")))?,
        None => with_verifier.with_no_client_auth(),
    };

    Ok(MakeRustlsConnect::new(client_config))
}
