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

/// upstream 인증서의 공개키 타입
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UpstreamKeyType {
    /// RSA 키 (비트 길이 포함)
    Rsa(u32),
    /// ECDSA 키 (곡선 종류 포함)
    Ecdsa(EcCurve),
    /// Ed25519 키
    Ed25519,
    /// 알 수 없는 키 타입
    Unknown,
}

/// ECDSA 곡선 종류
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EcCurve {
    P256,
    P384,
    P521,
}

impl Default for UpstreamKeyType {
    fn default() -> Self {
        UpstreamKeyType::Unknown
    }
}

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
    /// Issuer Common Name
    pub issuer_cn: Option<String>,
    /// 인증서 유효 시작일 (ISO 8601)
    pub not_before: Option<String>,
    /// 인증서 만료일 (ISO 8601)
    pub not_after: Option<String>,
    /// 시리얼 번호 (hex)
    pub serial_number: Option<String>,
    /// SHA-256 핑거프린트 (colon-separated hex)
    pub fingerprint_sha256: Option<String>,
    /// CA 인증서 여부
    pub is_ca: bool,
    /// 인증서 체인 길이 (end-entity 포함)
    pub chain_length: usize,
    /// 공개키 타입 (RSA, ECDSA, Ed25519)
    pub key_type: UpstreamKeyType,
}

impl UpstreamCertInfo {
    /// `ServerCertInfo`로 변환 (프론트엔드 전달용)
    pub fn to_server_cert_info(&self) -> proxy_v2_models::ServerCertInfo {
        proxy_v2_models::ServerCertInfo {
            subject_cn: self.common_name.clone(),
            issuer_cn: self.issuer_cn.clone(),
            organization: self.organization.clone(),
            sans_dns: self.sans_dns.clone(),
            sans_ip: self.sans_ip.iter().map(|ip| ip.to_string()).collect(),
            not_before: self.not_before.clone(),
            not_after: self.not_after.clone(),
            serial_number: self.serial_number.clone(),
            fingerprint_sha256: self.fingerprint_sha256.clone(),
            is_ca: self.is_ca,
            negotiated_alpn: self
                .negotiated_alpn
                .as_ref()
                .map(|a| String::from_utf8_lossy(a).to_string()),
            chain_length: self.chain_length,
        }
    }
}

/// 인증서를 캡처하는 TLS 검증기
#[derive(Debug)]
struct CertCapturingVerifier {
    captured: Arc<Mutex<Option<Vec<u8>>>>,
    chain_length: Arc<Mutex<usize>>,
}

impl ServerCertVerifier for CertCapturingVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, tokio_rustls::rustls::Error> {
        match self.captured.lock() {
            Ok(mut guard) => *guard = Some(end_entity.to_vec()),
            Err(poisoned) => *poisoned.into_inner() = Some(end_entity.to_vec()),
        }
        // 체인 길이: end-entity + intermediates
        match self.chain_length.lock() {
            Ok(mut guard) => *guard = 1 + intermediates.len(),
            Err(poisoned) => *poisoned.into_inner() = 1 + intermediates.len(),
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

/// ClientHello 미러링에 사용할 정보
#[derive(Debug, Clone)]
pub struct ClientHelloMirrorInfo {
    /// 클라이언트의 cipher suite 목록 (원본 순서)
    pub cipher_suites: Vec<u16>,
    /// Supported Groups (곡선 ID 목록)
    pub supported_groups: Vec<u16>,
    /// Signature Algorithms
    pub signature_algorithms: Vec<u16>,
    /// ALPN 프로토콜 목록
    pub alpn_protocols: Vec<Vec<u8>>,
}

/// 상류 서버에 연결하여 인증서 정보를 스니핑합니다.
///
/// `mirror_info`가 제공되면 OpenSSL을 사용하여 클라이언트의 TLS 설정을 미러링합니다.
/// 타임아웃 3초. 실패 시 None 반환 (호스트명 기반 인증서 생성으로 fallback).
pub async fn sniff_upstream_cert(
    authority: &Authority,
    upstream_proxy: Option<&UpstreamProxyConfig>,
) -> Option<UpstreamCertInfo> {
    sniff_upstream_cert_with_mirror(authority, upstream_proxy, None).await
}

/// ClientHello 미러링 정보를 포함한 upstream cert 스니핑
pub async fn sniff_upstream_cert_with_mirror(
    authority: &Authority,
    upstream_proxy: Option<&UpstreamProxyConfig>,
    mirror_info: Option<&ClientHelloMirrorInfo>,
) -> Option<UpstreamCertInfo> {
    let target_addr = format!(
        "{}:{}",
        authority.host(),
        authority
            .port()
            .map(|p| p.as_u16())
            .unwrap_or(DEFAULT_SSL_PORT)
    );

    debug!(
        "[UPSTREAM-CERT] 상류 인증서 스니핑 시작: {} (미러링: {})",
        target_addr,
        mirror_info.is_some()
    );

    #[cfg(feature = "openssl-ca")]
    let has_mirror = mirror_info.is_some();
    #[cfg(not(feature = "openssl-ca"))]
    let has_mirror = false;

    let result = if has_mirror {
        // OpenSSL 기반 미러링 스니핑 시도 → 실패 시 기존 rustls 폴백
        #[cfg(feature = "openssl-ca")]
        {
            let mirror = mirror_info.cloned();
            let authority_clone = authority.clone();
            let target_addr_clone = target_addr.clone();
            let upstream_proxy_cloned = upstream_proxy.map(|p| p.clone());

            tokio::time::timeout(std::time::Duration::from_secs(3), async move {
                if let Some(ref mi) = mirror {
                    if let Some(info) = sniff_upstream_cert_openssl_mirror(
                        &authority_clone,
                        &target_addr_clone,
                        upstream_proxy_cloned.as_ref(),
                        mi,
                    )
                    .await
                    {
                        return Some(info);
                    }
                    debug!("[UPSTREAM-CERT] OpenSSL 미러링 스니핑 실패, rustls 폴백");
                }
                sniff_upstream_cert_inner(
                    &authority_clone,
                    &target_addr_clone,
                    upstream_proxy_cloned.as_ref(),
                )
                .await
            })
            .await
        }
        #[cfg(not(feature = "openssl-ca"))]
        {
            tokio::time::timeout(
                std::time::Duration::from_secs(3),
                sniff_upstream_cert_inner(authority, &target_addr, upstream_proxy),
            )
            .await
        }
    } else {
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            sniff_upstream_cert_inner(authority, &target_addr, upstream_proxy),
        )
        .await
    };

    match result {
        Ok(Some(info)) => {
            info!(
                "[UPSTREAM-CERT] 스니핑 성공: {} (CN={:?}, DNS SANs={}개, IP SANs={}개, KeyType={:?})",
                authority,
                info.common_name,
                info.sans_dns.len(),
                info.sans_ip.len(),
                info.key_type
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
    let chain_length = Arc::new(Mutex::new(0usize));
    let verifier = CertCapturingVerifier {
        captured: Arc::clone(&captured),
        chain_length: Arc::clone(&chain_length),
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
    let captured_chain_length = *chain_length
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut info = parse_cert_info(&cert_der)?;
    info.negotiated_alpn = negotiated_alpn;
    info.chain_length = captured_chain_length;
    Some(info)
}

/// OpenSSL을 사용하여 클라이언트의 ClientHello를 미러링한 upstream cert 스니핑
///
/// 클라이언트가 보낸 cipher suite, supported groups, signature algorithms를
/// OpenSSL SSL_CTX에 설정하여 서버에 동일한 TLS 핑거프린트로 연결합니다.
#[cfg(feature = "openssl-ca")]
async fn sniff_upstream_cert_openssl_mirror(
    authority: &Authority,
    target_addr: &str,
    upstream_proxy: Option<&UpstreamProxyConfig>,
    mirror_info: &ClientHelloMirrorInfo,
) -> Option<UpstreamCertInfo> {
    use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
    use tokio_openssl::SslStream;

    // 1. TCP 연결
    let tcp_stream = match connect_to_target(target_addr, upstream_proxy).await {
        Ok(stream) => stream,
        Err(e) => {
            warn!(
                "[UPSTREAM-CERT-MIRROR] TCP 연결 실패: {} - {}",
                target_addr, e
            );
            return None;
        }
    };

    // 2. OpenSSL 설정 - ClientHello 미러링
    let mut connector_builder = SslConnector::builder(SslMethod::tls_client()).ok()?;

    // 인증서 검증 비활성화 (스니핑 목적)
    connector_builder.set_verify(SslVerifyMode::NONE);

    // cipher suite 미러링: 클라이언트의 원본 순서대로 설정
    if !mirror_info.cipher_suites.is_empty() {
        let cipher_string = build_openssl_cipher_string(&mirror_info.cipher_suites);
        if !cipher_string.is_empty() {
            let _ = connector_builder.set_cipher_list(&cipher_string);
        }
    }

    // ALPN 미러링
    if !mirror_info.alpn_protocols.is_empty() {
        let mut wire = Vec::new();
        for proto in &mirror_info.alpn_protocols {
            wire.push(proto.len() as u8);
            wire.extend_from_slice(proto);
        }
        let _ = connector_builder.set_alpn_protos(&wire);
    } else {
        let _ = connector_builder.set_alpn_protos(b"\x02h2\x08http/1.1");
    }

    // Supported Groups 미러링
    if !mirror_info.supported_groups.is_empty() {
        let groups_str = build_openssl_groups_string(&mirror_info.supported_groups);
        if !groups_str.is_empty() {
            let _ = connector_builder.set_groups_list(&groups_str);
        }
    }

    // Signature Algorithms 미러링
    if !mirror_info.signature_algorithms.is_empty() {
        let sigalg_string = build_openssl_sigalg_string(&mirror_info.signature_algorithms);
        if !sigalg_string.is_empty() {
            let _ = connector_builder.set_sigalgs_list(&sigalg_string);
        }
    }

    let ssl_connector = connector_builder.build();

    // IPv6 대괄호 제거
    let host = authority.host();
    let stripped_host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);

    let mut ssl_config = ssl_connector.configure().ok()?;
    // IP 주소인 경우 SNI 비활성화
    if stripped_host.parse::<std::net::IpAddr>().is_ok() {
        ssl_config.set_use_server_name_indication(false);
    }

    let ssl = ssl_config.into_ssl(stripped_host).ok()?;
    let mut tls_stream = SslStream::new(ssl, tcp_stream).ok()?;

    // 3. TLS 핸드셰이크
    let handshake_result = std::pin::Pin::new(&mut tls_stream).connect().await;

    let negotiated_alpn = match handshake_result {
        Ok(()) => {
            let ssl_ref = tls_stream.ssl();
            let alpn = ssl_ref.selected_alpn_protocol().map(|p| p.to_vec());
            debug!(
                "[UPSTREAM-CERT-MIRROR] ALPN 협상 결과: {:?}",
                alpn.as_ref()
                    .map(|p| String::from_utf8_lossy(p).to_string())
            );
            alpn
        }
        Err(e) => {
            debug!(
                "[UPSTREAM-CERT-MIRROR] TLS 핸드셰이크 실패: {} - {}",
                target_addr, e
            );
            // 핸드셰이크 실패해도 인증서는 캡처되었을 수 있음
            None
        }
    };

    // 4. 인증서 추출
    let ssl_ref = tls_stream.ssl();
    let peer_cert = ssl_ref.peer_certificate()?;
    let cert_der = peer_cert.to_der().ok()?;

    let chain_length = ssl_ref
        .peer_cert_chain()
        .map(|chain| chain.len())
        .unwrap_or(1);

    let mut info = parse_cert_info(&cert_der)?;
    info.negotiated_alpn = negotiated_alpn;
    info.chain_length = chain_length;

    info!(
        "[UPSTREAM-CERT-MIRROR] OpenSSL 미러링 스니핑 성공: {} (CN={:?}, KeyType={:?})",
        authority, info.common_name, info.key_type
    );

    Some(info)
}

/// TLS cipher suite ID를 OpenSSL cipher 문자열로 변환
#[cfg(feature = "openssl-ca")]
fn build_openssl_cipher_string(cipher_suites: &[u16]) -> String {
    // 일반적인 TLS cipher suite를 OpenSSL 이름으로 매핑
    // 완벽한 매핑이 아닌 "최선의 노력" 접근
    // 알려진 cipher만 포함하고, 알 수 없는 것은 무시
    let mut ciphers = Vec::new();

    for &suite in cipher_suites {
        if let Some(name) = tls_cipher_to_openssl_name(suite) {
            ciphers.push(name);
        }
    }

    if ciphers.is_empty() {
        // 미러링할 수 있는 cipher가 없으면 넓은 범위 사용
        return "ALL:!aNULL:!eNULL".to_string();
    }

    ciphers.join(":")
}

/// TLS cipher suite ID → OpenSSL cipher 이름
#[cfg(feature = "openssl-ca")]
fn tls_cipher_to_openssl_name(suite: u16) -> Option<&'static str> {
    match suite {
        // TLS 1.3 ciphers (ciphersuites, not cipher list)
        0x1301 => Some("TLS_AES_128_GCM_SHA256"),
        0x1302 => Some("TLS_AES_256_GCM_SHA384"),
        0x1303 => Some("TLS_CHACHA20_POLY1305_SHA256"),

        // ECDHE-ECDSA
        0xc02b => Some("ECDHE-ECDSA-AES128-GCM-SHA256"),
        0xc02c => Some("ECDHE-ECDSA-AES256-GCM-SHA384"),
        0xcca9 => Some("ECDHE-ECDSA-CHACHA20-POLY1305"),
        0xc009 => Some("ECDHE-ECDSA-AES128-SHA"),
        0xc00a => Some("ECDHE-ECDSA-AES256-SHA"),
        0xc023 => Some("ECDHE-ECDSA-AES128-SHA256"),
        0xc024 => Some("ECDHE-ECDSA-AES256-SHA384"),

        // ECDHE-RSA
        0xc02f => Some("ECDHE-RSA-AES128-GCM-SHA256"),
        0xc030 => Some("ECDHE-RSA-AES256-GCM-SHA384"),
        0xcca8 => Some("ECDHE-RSA-CHACHA20-POLY1305"),
        0xc013 => Some("ECDHE-RSA-AES128-SHA"),
        0xc014 => Some("ECDHE-RSA-AES256-SHA"),
        0xc027 => Some("ECDHE-RSA-AES128-SHA256"),
        0xc028 => Some("ECDHE-RSA-AES256-SHA384"),

        // DHE-RSA
        0x009e => Some("DHE-RSA-AES128-GCM-SHA256"),
        0x009f => Some("DHE-RSA-AES256-GCM-SHA384"),
        0x0033 => Some("DHE-RSA-AES128-SHA"),
        0x0039 => Some("DHE-RSA-AES256-SHA"),
        0x0067 => Some("DHE-RSA-AES128-SHA256"),
        0x006b => Some("DHE-RSA-AES256-SHA256"),
        0xccaa => Some("DHE-RSA-CHACHA20-POLY1305"),

        // RSA
        0x009c => Some("AES128-GCM-SHA256"),
        0x009d => Some("AES256-GCM-SHA384"),
        0x002f => Some("AES128-SHA"),
        0x0035 => Some("AES256-SHA"),
        0x003c => Some("AES128-SHA256"),
        0x003d => Some("AES256-SHA256"),

        // 기타
        0x00ff => None, // renegotiation_info - pseudo cipher
        0x0a0a | 0x1a1a | 0x2a2a | 0x3a3a | 0x4a4a | 0x5a5a | 0x6a6a | 0x7a7a | 0x8a8a | 0x9a9a
        | 0xaaaa | 0xbaba | 0xcaca | 0xdada | 0xeaea | 0xfafa => None, // GREASE values

        _ => None,
    }
}

/// TLS supported group ID → OpenSSL group 문자열
#[cfg(feature = "openssl-ca")]
fn build_openssl_groups_string(groups: &[u16]) -> String {
    let mut names = Vec::new();
    for &group in groups {
        if let Some(name) = tls_group_to_openssl_name(group) {
            names.push(name);
        }
    }
    names.join(":")
}

#[cfg(feature = "openssl-ca")]
fn tls_group_to_openssl_name(group: u16) -> Option<&'static str> {
    match group {
        0x0017 => Some("P-256"),  // secp256r1
        0x0018 => Some("P-384"),  // secp384r1
        0x0019 => Some("P-521"),  // secp521r1
        0x001d => Some("X25519"), // x25519
        0x001e => Some("X448"),   // x448
        // GREASE values
        0x0a0a | 0x1a1a | 0x2a2a | 0x3a3a | 0x4a4a | 0x5a5a | 0x6a6a | 0x7a7a | 0x8a8a | 0x9a9a
        | 0xaaaa | 0xbaba | 0xcaca | 0xdada | 0xeaea | 0xfafa => None,
        _ => None,
    }
}

/// TLS signature algorithm ID → OpenSSL sigalg 문자열
#[cfg(feature = "openssl-ca")]
fn build_openssl_sigalg_string(signature_algorithms: &[u16]) -> String {
    let mut sigalgs = Vec::new();

    for &alg in signature_algorithms {
        if let Some(name) = tls_sigalg_to_openssl_name(alg) {
            sigalgs.push(name);
        }
    }

    sigalgs.join(":")
}

#[cfg(feature = "openssl-ca")]
fn tls_sigalg_to_openssl_name(alg: u16) -> Option<&'static str> {
    match alg {
        0x0401 => Some("RSA+SHA256"),
        0x0501 => Some("RSA+SHA384"),
        0x0601 => Some("RSA+SHA512"),
        0x0403 => Some("ECDSA+SHA256"),
        0x0503 => Some("ECDSA+SHA384"),
        0x0603 => Some("ECDSA+SHA512"),
        0x0804 => Some("RSA-PSS+SHA256"),
        0x0805 => Some("RSA-PSS+SHA384"),
        0x0806 => Some("RSA-PSS+SHA512"),
        0x0807 => Some("ed25519"),
        0x0808 => Some("ed448"),
        // GREASE values
        0x0a0a | 0x1a1a | 0x2a2a | 0x3a3a | 0x4a4a | 0x5a5a | 0x6a6a | 0x7a7a | 0x8a8a | 0x9a9a
        | 0xaaaa | 0xbaba | 0xcaca | 0xdada | 0xeaea | 0xfafa => None,
        _ => None,
    }
}

/// DER 인코딩된 인증서에서 정보를 추출합니다
fn parse_cert_info(cert_der: &[u8]) -> Option<UpstreamCertInfo> {
    let (_, cert) = x509_parser::parse_x509_certificate(cert_der).ok()?;

    let mut info = UpstreamCertInfo::default();

    // Subject: CN, Organization 추출
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

    // Issuer: CN 추출
    for rdn in cert.issuer().iter() {
        for attr in rdn.iter() {
            if *attr.attr_type() == OID_X509_COMMON_NAME {
                info.issuer_cn = attr.as_str().ok().map(String::from);
            }
        }
    }

    // 유효기간 (ASN.1 GeneralizedTime → ISO 8601)
    info.not_before = Some(
        cert.validity()
            .not_before
            .to_rfc2822()
            .unwrap_or_else(|_| format!("{}", cert.validity().not_before)),
    );
    info.not_after = Some(
        cert.validity()
            .not_after
            .to_rfc2822()
            .unwrap_or_else(|_| format!("{}", cert.validity().not_after)),
    );

    // Serial Number (hex)
    info.serial_number = Some(
        cert.raw_serial()
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(":"),
    );

    // SHA-256 Fingerprint
    use sha2::{Digest, Sha256};
    let fingerprint = Sha256::digest(cert_der);
    info.fingerprint_sha256 = Some(
        fingerprint
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(":"),
    );

    // CA 여부 (Basic Constraints)
    info.is_ca = cert
        .basic_constraints()
        .ok()
        .flatten()
        .map(|bc| bc.value.ca)
        .unwrap_or(false);

    // 공개키 타입 추출
    info.key_type = parse_key_type(&cert);

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
        "[UPSTREAM-CERT] 파싱 결과: CN={:?}, Issuer={:?}, Org={:?}, DNS SANs={:?}, IP SANs={:?}, Not After={:?}, KeyType={:?}",
        info.common_name,
        info.issuer_cn,
        info.organization,
        info.sans_dns,
        info.sans_ip,
        info.not_after,
        info.key_type
    );

    Some(info)
}

/// X.509 인증서에서 공개키 타입을 추출합니다
fn parse_key_type(cert: &x509_parser::certificate::X509Certificate) -> UpstreamKeyType {
    use x509_parser::oid_registry::{
        OID_KEY_TYPE_EC_PUBLIC_KEY, OID_PKCS1_RSAENCRYPTION, OID_SIG_ED25519,
    };

    let spki = cert.public_key();
    let algorithm_oid = spki.algorithm.algorithm.clone();

    if algorithm_oid == OID_PKCS1_RSAENCRYPTION {
        // RSA: 공개키에서 비트 길이 추출
        let bit_size = spki.parsed().ok().map(|pk| {
            if let x509_parser::public_key::PublicKey::RSA(rsa) = pk {
                (rsa.modulus.len() as u32) * 8
            } else {
                2048 // 기본값
            }
        });
        UpstreamKeyType::Rsa(bit_size.unwrap_or(2048))
    } else if algorithm_oid == OID_KEY_TYPE_EC_PUBLIC_KEY {
        // ECDSA: 곡선 OID로 곡선 종류 결정
        let curve = spki
            .algorithm
            .parameters
            .as_ref()
            .and_then(|params| params.as_oid().ok())
            .map(|curve_oid| {
                // NIST P-256: 1.2.840.10045.3.1.7
                // NIST P-384: 1.3.132.0.34
                // NIST P-521: 1.3.132.0.35
                let oid_str = curve_oid.to_id_string();
                match oid_str.as_str() {
                    "1.2.840.10045.3.1.7" => EcCurve::P256,
                    "1.3.132.0.34" => EcCurve::P384,
                    "1.3.132.0.35" => EcCurve::P521,
                    _ => EcCurve::P256, // 기본값
                }
            })
            .unwrap_or(EcCurve::P256);
        UpstreamKeyType::Ecdsa(curve)
    } else if algorithm_oid == OID_SIG_ED25519 {
        UpstreamKeyType::Ed25519
    } else {
        debug!("[UPSTREAM-CERT] 알 수 없는 키 타입 OID: {}", algorithm_oid);
        UpstreamKeyType::Unknown
    }
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
            ..Default::default()
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

    #[test]
    fn test_to_server_cert_info_basic_conversion() {
        let info = UpstreamCertInfo {
            common_name: Some("example.com".to_string()),
            organization: Some("Example Inc.".to_string()),
            sans_dns: vec!["example.com".to_string(), "*.example.com".to_string()],
            sans_ip: vec!["127.0.0.1".parse().unwrap(), "::1".parse().unwrap()],
            negotiated_alpn: Some(b"h2".to_vec()),
            issuer_cn: Some("Let's Encrypt".to_string()),
            not_before: Some("2024-01-01T00:00:00Z".to_string()),
            not_after: Some("2025-01-01T00:00:00Z".to_string()),
            serial_number: Some("01:AB:CD".to_string()),
            fingerprint_sha256: Some("AA:BB:CC".to_string()),
            is_ca: false,
            chain_length: 3,
            key_type: UpstreamKeyType::Ecdsa(EcCurve::P256),
        };

        let cert_info = info.to_server_cert_info();

        assert_eq!(cert_info.subject_cn, Some("example.com".to_string()));
        assert_eq!(cert_info.organization, Some("Example Inc.".to_string()));
        assert_eq!(cert_info.issuer_cn, Some("Let's Encrypt".to_string()));
        assert_eq!(cert_info.sans_dns.len(), 2);
        assert_eq!(cert_info.sans_ip, vec!["127.0.0.1", "::1"]);
        assert_eq!(
            cert_info.not_before,
            Some("2024-01-01T00:00:00Z".to_string())
        );
        assert_eq!(
            cert_info.not_after,
            Some("2025-01-01T00:00:00Z".to_string())
        );
        assert_eq!(cert_info.serial_number, Some("01:AB:CD".to_string()));
        assert_eq!(cert_info.fingerprint_sha256, Some("AA:BB:CC".to_string()));
        assert!(!cert_info.is_ca);
        assert_eq!(cert_info.negotiated_alpn, Some("h2".to_string()));
        assert_eq!(cert_info.chain_length, 3);
    }

    #[test]
    fn test_to_server_cert_info_default_values() {
        let info = UpstreamCertInfo::default();
        let cert_info = info.to_server_cert_info();

        assert!(cert_info.subject_cn.is_none());
        assert!(cert_info.issuer_cn.is_none());
        assert!(cert_info.organization.is_none());
        assert!(cert_info.sans_dns.is_empty());
        assert!(cert_info.sans_ip.is_empty());
        assert!(cert_info.not_before.is_none());
        assert!(cert_info.not_after.is_none());
        assert!(cert_info.serial_number.is_none());
        assert!(cert_info.fingerprint_sha256.is_none());
        assert!(!cert_info.is_ca);
        assert!(cert_info.negotiated_alpn.is_none());
        assert_eq!(cert_info.chain_length, 0);
    }

    #[test]
    fn test_to_server_cert_info_ip_addr_to_string() {
        let info = UpstreamCertInfo {
            sans_ip: vec![
                "192.168.1.1".parse().unwrap(),
                "2001:db8::1".parse().unwrap(),
            ],
            ..Default::default()
        };

        let cert_info = info.to_server_cert_info();
        assert_eq!(cert_info.sans_ip, vec!["192.168.1.1", "2001:db8::1"]);
    }

    #[test]
    fn test_upstream_key_type_default_is_unknown() {
        let kt = UpstreamKeyType::default();
        assert_eq!(kt, UpstreamKeyType::Unknown);
    }

    #[test]
    fn test_upstream_cert_info_default_key_type() {
        let info = UpstreamCertInfo::default();
        assert_eq!(info.key_type, UpstreamKeyType::Unknown);
    }

    #[test]
    fn test_client_hello_mirror_info_construction() {
        let mirror = ClientHelloMirrorInfo {
            cipher_suites: vec![0xc02f, 0xc030, 0x1301],
            supported_groups: vec![0x0017, 0x0018, 0x001d],
            signature_algorithms: vec![0x0403, 0x0804],
            alpn_protocols: vec![b"h2".to_vec(), b"http/1.1".to_vec()],
        };
        assert_eq!(mirror.cipher_suites.len(), 3);
        assert_eq!(mirror.supported_groups.len(), 3);
        assert_eq!(mirror.signature_algorithms.len(), 2);
        assert_eq!(mirror.alpn_protocols.len(), 2);
    }

    #[cfg(feature = "openssl-ca")]
    mod openssl_mirror_tests {
        use super::*;

        #[test]
        fn test_build_openssl_cipher_string_known_ciphers() {
            let ciphers = vec![0xc02f, 0xc030, 0xcca8];
            let result = build_openssl_cipher_string(&ciphers);
            assert!(result.contains("ECDHE-RSA-AES128-GCM-SHA256"));
            assert!(result.contains("ECDHE-RSA-AES256-GCM-SHA384"));
            assert!(result.contains("ECDHE-RSA-CHACHA20-POLY1305"));
        }

        #[test]
        fn test_build_openssl_cipher_string_grease_values_ignored() {
            let ciphers = vec![0x0a0a, 0xcaca, 0xfafa, 0xc02f];
            let result = build_openssl_cipher_string(&ciphers);
            // GREASE 값은 무시되고 실제 cipher만 포함
            assert!(result.contains("ECDHE-RSA-AES128-GCM-SHA256"));
            assert!(!result.contains("0a0a"));
        }

        #[test]
        fn test_build_openssl_cipher_string_empty_fallback() {
            let ciphers = vec![0x0a0a, 0xfafa]; // GREASE만
            let result = build_openssl_cipher_string(&ciphers);
            assert_eq!(result, "ALL:!aNULL:!eNULL");
        }

        #[test]
        fn test_build_openssl_cipher_string_tls13() {
            let ciphers = vec![0x1301, 0x1302, 0x1303];
            let result = build_openssl_cipher_string(&ciphers);
            assert!(result.contains("TLS_AES_128_GCM_SHA256"));
            assert!(result.contains("TLS_AES_256_GCM_SHA384"));
            assert!(result.contains("TLS_CHACHA20_POLY1305_SHA256"));
        }

        #[test]
        fn test_build_openssl_groups_string() {
            let groups = vec![0x0017, 0x0018, 0x001d, 0x0a0a];
            let result = build_openssl_groups_string(&groups);
            assert!(result.contains("P-256"));
            assert!(result.contains("P-384"));
            assert!(result.contains("X25519"));
            assert!(!result.contains("0a0a"));
        }

        #[test]
        fn test_build_openssl_sigalg_string() {
            let sigalgs = vec![0x0403, 0x0804, 0x0807];
            let result = build_openssl_sigalg_string(&sigalgs);
            assert!(result.contains("ECDSA+SHA256"));
            assert!(result.contains("RSA-PSS+SHA256"));
            assert!(result.contains("ed25519"));
        }

        #[test]
        fn test_tls_cipher_to_openssl_name_coverage() {
            // ECDHE-ECDSA 계열
            assert_eq!(
                tls_cipher_to_openssl_name(0xc02b),
                Some("ECDHE-ECDSA-AES128-GCM-SHA256")
            );
            assert_eq!(
                tls_cipher_to_openssl_name(0xcca9),
                Some("ECDHE-ECDSA-CHACHA20-POLY1305")
            );
            // RSA 계열
            assert_eq!(
                tls_cipher_to_openssl_name(0x009c),
                Some("AES128-GCM-SHA256")
            );
            // DHE 계열
            assert_eq!(
                tls_cipher_to_openssl_name(0x009e),
                Some("DHE-RSA-AES128-GCM-SHA256")
            );
            // renegotiation_info는 None
            assert_eq!(tls_cipher_to_openssl_name(0x00ff), None);
            // 알 수 없는 cipher
            assert_eq!(tls_cipher_to_openssl_name(0xffff), None);
        }

        #[test]
        fn test_tls_group_to_openssl_name_coverage() {
            assert_eq!(tls_group_to_openssl_name(0x0017), Some("P-256"));
            assert_eq!(tls_group_to_openssl_name(0x0019), Some("P-521"));
            assert_eq!(tls_group_to_openssl_name(0x001e), Some("X448"));
            assert_eq!(tls_group_to_openssl_name(0x0a0a), None); // GREASE
            assert_eq!(tls_group_to_openssl_name(0x9999), None); // unknown
        }
    }
}
