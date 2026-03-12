use std::sync::Arc;
use tokio_rustls::rustls::{
    client::ResolvesClientCert,
    crypto::aws_lc_rs,
    pki_types::{CertificateDer, PrivateKeyDer},
    sign::CertifiedKey,
    SignatureScheme,
};

use super::loader::{load_certs, load_private_key};

/// 인증서와 키 파일에서 CertifiedKey를 빌드합니다.
pub(super) fn build_certified_key(
    cert_path: &str,
    key_path: &str,
) -> Result<CertifiedKey, Box<dyn std::error::Error>> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;
    let signing_key = aws_lc_rs::sign::any_supported_type(&key)?;
    Ok(CertifiedKey::new(certs, signing_key))
}

/// 클라이언트 인증서 리졸버
///
/// NOTE: `ResolvesClientCert` 트레잇은 서버의 CertificateRequest에서 acceptable CA 목록만
/// 제공하므로 도메인 기반 매칭이 불가합니다. 도메인별 인증서는 향후 per-connection
/// ClientConfig 방식으로 확장 예정입니다. 현재는 기본 인증서만 반환합니다.
#[derive(Debug)]
pub(super) struct DefaultCertResolver {
    /// 기본 인증서 (글로벌 설정)
    pub(super) default_cert: Option<Arc<CertifiedKey>>,
}

impl ResolvesClientCert for DefaultCertResolver {
    fn resolve(
        &self,
        _root_hint_subjects: &[&[u8]],
        _sigschemes: &[SignatureScheme],
    ) -> Option<Arc<CertifiedKey>> {
        self.default_cert.clone()
    }

    fn has_certs(&self) -> bool {
        self.default_cert.is_some()
    }
}
