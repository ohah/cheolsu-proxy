use crate::upstream_proxy::{UpstreamProxyConfig, connect_to_target};
use http::uri::Authority;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use tokio_rustls::rustls::{
    ClientConfig, DigitallySignedStruct, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use tracing::{debug, info, warn};
use x509_parser::extensions::GeneralName;

/// SSL/TLS 기본 포트 (HTTPS)
const DEFAULT_SSL_PORT: u16 = 443;
use x509_parser::oid_registry::{OID_X509_COMMON_NAME, OID_X509_ORGANIZATION_NAME};

/// 상류 서버 인증서에서 추출한 정보
#[derive(Debug, Clone, Default)]
pub struct UpstreamCertInfo {
    /// Common Name
    pub common_name: Option<String>,
    /// 조직명
    pub organization: Option<String>,
    /// DNS SAN 목록
    pub sans_dns: Vec<String>,
    /// IP SAN 목록
    pub sans_ip: Vec<IpAddr>,
    /// 상류 서버가 지원하는 ALPN 프로토콜 (협상 결과)
    pub negotiated_alpn: Option<Vec<u8>>,
}

/// 인증서를 캡처하는 TLS 검증기
#[derive(Debug)]
struct CertCapturingVerifier {
    captured: Arc<Mutex<Option<Vec<u8>>>>,
}

impl ServerCertVerifier for CertCapturingVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, tokio_rustls::rustls::Error> {
        match self.captured.lock() {
            Ok(mut guard) => *guard = Some(end_entity.to_vec()),
            Err(poisoned) => *poisoned.into_inner() = Some(end_entity.to_vec()),
        }
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
        ]
    }
}

/// 상류 서버에 연결하여 인증서 정보를 스니핑합니다.
///
/// 타임아웃 3초. 실패 시 None 반환 (호스트명 기반 인증서 생성으로 fallback).
pub async fn sniff_upstream_cert(
    authority: &Authority,
    upstream_proxy: Option<&UpstreamProxyConfig>,
) -> Option<UpstreamCertInfo> {
    let target_addr = format!(
        "{}:{}",
        authority.host(),
        authority
            .port()
            .map(|p| p.as_u16())
            .unwrap_or(DEFAULT_SSL_PORT)
    );

    debug!("[UPSTREAM-CERT] 상류 인증서 스니핑 시작: {}", target_addr);

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        sniff_upstream_cert_inner(authority, &target_addr, upstream_proxy),
    )
    .await;

    match result {
        Ok(Some(info)) => {
            info!(
                "[UPSTREAM-CERT] 스니핑 성공: {} (CN={:?}, DNS SANs={}개, IP SANs={}개)",
                authority,
                info.common_name,
                info.sans_dns.len(),
                info.sans_ip.len()
            );
            Some(info)
        }
        Ok(None) => {
            warn!("[UPSTREAM-CERT] 스니핑 실패: {}", authority);
            None
        }
        Err(_) => {
            warn!("[UPSTREAM-CERT] 스니핑 타임아웃 (3초): {}", authority);
            None
        }
    }
}

async fn sniff_upstream_cert_inner(
    authority: &Authority,
    target_addr: &str,
    upstream_proxy: Option<&UpstreamProxyConfig>,
) -> Option<UpstreamCertInfo> {
    // 1. TCP 연결
    let tcp_stream = match connect_to_target(target_addr, upstream_proxy).await {
        Ok(stream) => stream,
        Err(e) => {
            warn!("[UPSTREAM-CERT] TCP 연결 실패: {} - {}", target_addr, e);
            return None;
        }
    };

    // 2. 인증서 캡처용 TLS 설정
    let captured = Arc::new(Mutex::new(None));
    let verifier = CertCapturingVerifier {
        captured: Arc::clone(&captured),
    };

    let mut config = ClientConfig::builder_with_provider(Arc::new(
        tokio_rustls::rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .ok()?
    .dangerous()
    .with_custom_certificate_verifier(Arc::new(verifier))
    .with_no_client_auth();

    // ALPN 프로토콜 설정 - 서버가 어떤 프로토콜을 지원하는지 확인
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));

    // IPv6 대괄호 제거 및 IP 주소 처리 (RFC 6066: IP literal은 SNI에 불허)
    let host = authority.host();
    let stripped_host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);

    let server_name = if let Ok(ip_addr) = stripped_host.parse::<std::net::IpAddr>() {
        // IP 주소인 경우 ServerName::IpAddress 사용 → rustls가 SNI를 전송하지 않음
        debug!(
            "[UPSTREAM-CERT] IP 주소 감지, SNI 미전송 (RFC 6066): {}",
            ip_addr
        );
        ServerName::from(ip_addr)
    } else {
        ServerName::try_from(stripped_host.to_string()).ok()?
    };

    // 3. TLS 핸드셰이크 (인증서 + ALPN 캡처)
    let negotiated_alpn = match connector.connect(server_name, tcp_stream).await {
        Ok(tls_stream) => {
            // 협상된 ALPN 프로토콜 캡처
            let alpn = tls_stream.get_ref().1.alpn_protocol().map(|p| p.to_vec());
            debug!(
                "[UPSTREAM-CERT] ALPN 협상 결과: {:?}",
                alpn.as_ref()
                    .map(|p| String::from_utf8_lossy(p).to_string())
            );
            alpn
        }
        Err(e) => {
            debug!(
                "[UPSTREAM-CERT] TLS 핸드셰이크 실패 (인증서는 캡처되었을 수 있음): {} - {}",
                target_addr, e
            );
            None
        }
    };

    // 4. 캡처된 인증서 파싱
    let cert_der = captured
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()?;
    let mut info = parse_cert_info(&cert_der)?;
    info.negotiated_alpn = negotiated_alpn;
    Some(info)
}

/// DER 인코딩된 인증서에서 정보를 추출합니다
fn parse_cert_info(cert_der: &[u8]) -> Option<UpstreamCertInfo> {
    let (_, cert) = x509_parser::parse_x509_certificate(cert_der).ok()?;

    let mut info = UpstreamCertInfo::default();

    // CN, Organization 추출
    for rdn in cert.subject().iter() {
        for attr in rdn.iter() {
            if *attr.attr_type() == OID_X509_COMMON_NAME {
                info.common_name = attr.as_str().ok().map(String::from);
            }
            if *attr.attr_type() == OID_X509_ORGANIZATION_NAME {
                info.organization = attr.as_str().ok().map(String::from);
            }
        }
    }

    // SAN 추출
    if let Ok(Some(san_ext)) = cert.subject_alternative_name() {
        for name in &san_ext.value.general_names {
            match name {
                GeneralName::DNSName(dns) => {
                    info.sans_dns.push(dns.to_string());
                }
                GeneralName::IPAddress(ip_bytes) => {
                    if ip_bytes.len() == 4 {
                        let ip = IpAddr::from([ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]]);
                        info.sans_ip.push(ip);
                    } else if ip_bytes.len() == 16 {
                        let mut octets = [0u8; 16];
                        octets.copy_from_slice(ip_bytes);
                        info.sans_ip.push(IpAddr::from(octets));
                    }
                }
                _ => {}
            }
        }
    }

    debug!(
        "[UPSTREAM-CERT] 파싱 결과: CN={:?}, Org={:?}, DNS SANs={:?}, IP SANs={:?}",
        info.common_name, info.organization, info.sans_dns, info.sans_ip
    );

    Some(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upstream_cert_info_default() {
        let info = UpstreamCertInfo::default();
        assert!(info.common_name.is_none());
        assert!(info.organization.is_none());
        assert!(info.sans_dns.is_empty());
        assert!(info.sans_ip.is_empty());
    }

    #[test]
    fn test_upstream_cert_info_clone() {
        let info = UpstreamCertInfo {
            common_name: Some("example.com".to_string()),
            organization: Some("Example Inc.".to_string()),
            sans_dns: vec!["example.com".to_string(), "*.example.com".to_string()],
            sans_ip: vec!["127.0.0.1".parse().unwrap()],
            negotiated_alpn: Some(b"h2".to_vec()),
        };
        let cloned = info.clone();
        assert_eq!(cloned.common_name, info.common_name);
        assert_eq!(cloned.sans_dns.len(), 2);
        assert_eq!(cloned.sans_ip.len(), 1);
    }

    #[test]
    fn test_parse_cert_info_invalid_der() {
        let result = parse_cert_info(b"not a valid certificate");
        assert!(result.is_none());
    }
}
