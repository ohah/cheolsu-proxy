use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use proxyapi_v2::{
    upstream_proxy::{ProxyHttpConnector, UpstreamProxyConfig},
    Body,
};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;
use tokio_rustls::rustls::{
    client::ResolvesClientCert,
    crypto::aws_lc_rs,
    pki_types::{CertificateDer, PrivateKeyDer},
    sign::CertifiedKey,
    ClientConfig, SignatureScheme,
};
use tracing::{error, info};
use x509_parser::extensions::{GeneralName, ParsedExtension};
use x509_parser::oid_registry::{OID_X509_COMMON_NAME, OID_X509_ORGANIZATION_NAME};

use crate::protocol::{CertificateInfo, ClientCertConfig};

/// 모든 인증서를 허용하는 위험한 인증서 검증기
#[derive(Debug)]
struct DangerousCertificateVerifier;

impl tokio_rustls::rustls::client::danger::ServerCertVerifier for DangerousCertificateVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[tokio_rustls::rustls::pki_types::CertificateDer<'_>],
        _server_name: &tokio_rustls::rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: tokio_rustls::rustls::pki_types::UnixTime,
    ) -> Result<tokio_rustls::rustls::client::danger::ServerCertVerified, tokio_rustls::rustls::Error>
    {
        Ok(tokio_rustls::rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        tokio_rustls::rustls::Error,
    > {
        Ok(tokio_rustls::rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        tokio_rustls::rustls::Error,
    > {
        Ok(tokio_rustls::rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<tokio_rustls::rustls::SignatureScheme> {
        vec![
            tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA1,
            tokio_rustls::rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA256,
            tokio_rustls::rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA384,
            tokio_rustls::rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA512,
            tokio_rustls::rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            tokio_rustls::rustls::SignatureScheme::RSA_PSS_SHA256,
            tokio_rustls::rustls::SignatureScheme::RSA_PSS_SHA384,
            tokio_rustls::rustls::SignatureScheme::RSA_PSS_SHA512,
            tokio_rustls::rustls::SignatureScheme::ED25519,
            tokio_rustls::rustls::SignatureScheme::ED448,
            tokio_rustls::rustls::SignatureScheme::ML_DSA_44,
            tokio_rustls::rustls::SignatureScheme::ML_DSA_65,
            tokio_rustls::rustls::SignatureScheme::ML_DSA_87,
        ]
    }
}

/// 인증서와 키 파일에서 CertifiedKey를 빌드합니다.
fn build_certified_key(
    cert_path: &str,
    key_path: &str,
) -> Result<CertifiedKey, Box<dyn std::error::Error>> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;
    let signing_key = aws_lc_rs::sign::any_supported_type(&key)?;
    Ok(CertifiedKey::new(certs, signing_key))
}

/// 도메인 패턴별 클라이언트 인증서를 선택하는 리졸버
#[derive(Debug)]
struct DomainCertResolver {
    /// 기본 인증서 (글로벌 설정)
    default_cert: Option<Arc<CertifiedKey>>,
    /// 도메인 패턴 -> 인증서 매핑
    domain_certs: Vec<(String, Arc<CertifiedKey>)>,
}

impl ResolvesClientCert for DomainCertResolver {
    fn resolve(
        &self,
        _root_hint_subjects: &[&[u8]],
        _sigschemes: &[SignatureScheme],
    ) -> Option<Arc<CertifiedKey>> {
        // NOTE: ResolvesClientCert는 서버가 CertificateRequest를 보낼 때 호출됨
        // 서버의 acceptable CA 목록(root_hint_subjects)으로는 도메인 매칭이 불가하므로
        // 기본 인증서를 반환. 도메인별 인증서는 per-connection ClientConfig으로 확장 가능.
        self.default_cert.clone()
    }

    fn has_certs(&self) -> bool {
        self.default_cert.is_some() || !self.domain_certs.is_empty()
    }
}

/// 인증서 체인을 로드합니다. PEM 형식을 먼저 시도하고, 실패 시 DER 형식으로 시도합니다.
pub fn load_certs(
    cert_path: &str,
) -> Result<Vec<CertificateDer<'static>>, Box<dyn std::error::Error>> {
    let cert_data = std::fs::read(cert_path)?;

    // PEM 형식 시도
    let certs: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut cert_data.as_slice()).collect::<Result<Vec<_>, _>>()?;

    if certs.is_empty() {
        // DER 형식으로 시도 - 최소한 ASN.1 시퀀스 태그(0x30)인지 확인
        if cert_data.first().copied() != Some(0x30) {
            return Err("인증서 파일이 유효한 PEM 또는 DER 형식이 아닙니다".into());
        }
        return Ok(vec![CertificateDer::from(cert_data)]);
    }

    Ok(certs)
}

/// 인증서 파일에서 상세 정보를 추출합니다.
pub fn parse_certificate_info(
    cert_path: &str,
) -> Result<CertificateInfo, Box<dyn std::error::Error>> {
    let certs = load_certs(cert_path)?;
    if certs.is_empty() {
        return Err("인증서 파일에서 인증서를 찾을 수 없습니다".into());
    }

    let chain_length = certs.len();
    let cert_der = certs[0].as_ref();

    // x509-parser로 인증서 파싱
    let (_, cert) = x509_parser::parse_x509_certificate(cert_der)
        .map_err(|e| format!("인증서 파싱 실패: {}", e))?;

    // Subject CN, Organization 추출
    let mut subject_cn = None;
    let mut organization = None;
    for rdn in cert.subject().iter() {
        for attr in rdn.iter() {
            if *attr.attr_type() == OID_X509_COMMON_NAME {
                subject_cn = attr.as_str().ok().map(String::from);
            }
            if *attr.attr_type() == OID_X509_ORGANIZATION_NAME {
                organization = attr.as_str().ok().map(String::from);
            }
        }
    }

    // Issuer CN 추출
    let mut issuer_cn = None;
    for rdn in cert.issuer().iter() {
        for attr in rdn.iter() {
            if *attr.attr_type() == OID_X509_COMMON_NAME {
                issuer_cn = attr.as_str().ok().map(String::from);
            }
        }
    }

    // SAN 추출
    let mut sans_dns = Vec::new();
    let mut sans_ip = Vec::new();
    if let Ok(Some(san_ext)) = cert.subject_alternative_name() {
        for name in &san_ext.value.general_names {
            match name {
                GeneralName::DNSName(dns) => {
                    sans_dns.push(dns.to_string());
                }
                GeneralName::IPAddress(ip_bytes) => {
                    if ip_bytes.len() == 4 {
                        let ip = format!(
                            "{}.{}.{}.{}",
                            ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]
                        );
                        sans_ip.push(ip);
                    } else if ip_bytes.len() == 16 {
                        let addr: std::net::Ipv6Addr = {
                            let mut octets = [0u8; 16];
                            octets.copy_from_slice(ip_bytes);
                            std::net::Ipv6Addr::from(octets)
                        };
                        sans_ip.push(addr.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    // 유효기간
    let not_before = cert
        .validity()
        .not_before
        .to_rfc2822()
        .unwrap_or_else(|_| format!("{}", cert.validity().not_before));
    let not_after = cert
        .validity()
        .not_after
        .to_rfc2822()
        .unwrap_or_else(|_| format!("{}", cert.validity().not_after));

    // 시리얼 넘버
    let serial_number = cert.raw_serial_as_string();

    // SHA-256 지문
    let hash = Sha256::digest(cert_der);
    let fingerprint_sha256 = hash
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(":");

    // CA 여부 (BasicConstraints 확인)
    let is_ca = cert
        .extensions()
        .iter()
        .find_map(|ext| {
            if let ParsedExtension::BasicConstraints(bc) = ext.parsed_extension() {
                Some(bc.ca)
            } else {
                None
            }
        })
        .unwrap_or(false);

    Ok(CertificateInfo {
        subject_cn,
        issuer_cn,
        organization,
        sans_dns,
        sans_ip,
        not_before,
        not_after,
        serial_number,
        fingerprint_sha256,
        is_ca,
        chain_length,
    })
}

/// PEM 파일에서 인증서 체인을 로드합니다 (하위 호환성).
#[deprecated(note = "load_certs를 사용하세요")]
pub fn load_certs_from_pem(
    cert_path: &str,
) -> Result<Vec<CertificateDer<'static>>, Box<dyn std::error::Error>> {
    load_certs(cert_path)
}

/// PEM 파일에서 개인 키를 로드합니다. RSA (PKCS#1), PKCS#8, EC (SEC1) 키를 지원합니다.
/// 현재 DER 형식 키는 지원하지 않습니다.
pub fn load_private_key(
    key_path: &str,
) -> Result<PrivateKeyDer<'static>, Box<dyn std::error::Error>> {
    let key_data = std::fs::read(key_path)?;
    let mut reader = key_data.as_slice();

    // rustls_pemfile를 사용하여 다양한 PEM 키 형식 파싱
    loop {
        match rustls_pemfile::read_one(&mut reader)? {
            Some(rustls_pemfile::Item::Pkcs1Key(key)) => {
                return Ok(PrivateKeyDer::Pkcs1(key));
            }
            Some(rustls_pemfile::Item::Pkcs8Key(key)) => {
                return Ok(PrivateKeyDer::Pkcs8(key));
            }
            Some(rustls_pemfile::Item::Sec1Key(key)) => {
                return Ok(PrivateKeyDer::Sec1(key));
            }
            Some(_) => continue, // 다른 PEM 항목은 건너뜀
            None => break,
        }
    }

    Err("키 파일에서 유효한 개인 키를 찾을 수 없습니다".into())
}

/// PEM 파일에서 개인 키를 로드합니다 (하위 호환성).
#[deprecated(note = "load_private_key를 사용하세요")]
pub fn load_private_key_from_pem(
    key_path: &str,
) -> Result<PrivateKeyDer<'static>, Box<dyn std::error::Error>> {
    load_private_key(key_path)
}

/// 클라이언트 인증서 설정을 검증합니다.
pub fn validate_client_cert_config(
    config: &ClientCertConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if !config.enabled {
        return Ok(());
    }

    let cert_path = Path::new(&config.cert_path);
    if !cert_path.exists() {
        return Err(format!("인증서 파일이 존재하지 않습니다: {}", config.cert_path).into());
    }

    let key_path = Path::new(&config.key_path);
    if !key_path.exists() {
        return Err(format!("키 파일이 존재하지 않습니다: {}", config.key_path).into());
    }

    // 실제 로드 테스트
    load_certs(&config.cert_path)?;
    load_private_key(&config.key_path)?;

    Ok(())
}

/// 하이브리드 클라이언트 생성 (모든 인증서 허용, upstream proxy 지원)
///
/// `upstream_rx`를 통해 런타임에 upstream proxy 설정 변경이 즉시 반영됩니다.
pub fn create_hybrid_client(
    upstream_rx: tokio::sync::watch::Receiver<Option<UpstreamProxyConfig>>,
) -> Result<
    Client<hyper_rustls::HttpsConnector<ProxyHttpConnector>, Body>,
    Box<dyn std::error::Error>,
> {
    create_hybrid_client_with_cert(upstream_rx, None)
}

/// 클라이언트 인증서를 포함한 하이브리드 클라이언트 생성
pub fn create_hybrid_client_with_cert(
    upstream_rx: tokio::sync::watch::Receiver<Option<UpstreamProxyConfig>>,
    client_cert_config: Option<&ClientCertConfig>,
) -> Result<
    Client<hyper_rustls::HttpsConnector<ProxyHttpConnector>, Body>,
    Box<dyn std::error::Error>,
> {
    let config_builder =
        ClientConfig::builder_with_provider(std::sync::Arc::new(aws_lc_rs::default_provider()))
            .with_safe_default_protocol_versions()?
            .dangerous()
            .with_custom_certificate_verifier(std::sync::Arc::new(DangerousCertificateVerifier));

    let rustls_config = if let Some(cert_config) = client_cert_config {
        if cert_config.enabled {
            // 기본 인증서 로드
            let default_cert =
                match build_certified_key(&cert_config.cert_path, &cert_config.key_path) {
                    Ok(key) => {
                        info!(
                            "기본 클라이언트 인증서 로드 성공: cert={}, key={}",
                            cert_config.cert_path, cert_config.key_path
                        );
                        Some(Arc::new(key))
                    }
                    Err(e) => {
                        error!("기본 클라이언트 인증서 로드 실패: {}", e);
                        None
                    }
                };

            // 도메인별 인증서 로드
            let mut domain_certs = Vec::new();
            for dc in &cert_config.domain_certs {
                if !dc.enabled {
                    continue;
                }
                match build_certified_key(&dc.cert_path, &dc.key_path) {
                    Ok(key) => {
                        info!(
                            "도메인 인증서 로드 성공: pattern={}, cert={}",
                            dc.domain_pattern, dc.cert_path
                        );
                        domain_certs.push((dc.domain_pattern.clone(), Arc::new(key)));
                    }
                    Err(e) => {
                        error!("도메인 인증서 로드 실패 ({}): {}", dc.domain_pattern, e);
                    }
                }
            }

            let resolver = DomainCertResolver {
                default_cert,
                domain_certs,
            };
            config_builder.with_client_cert_resolver(Arc::new(resolver))
        } else {
            config_builder.with_no_client_auth()
        }
    } else {
        config_builder.with_no_client_auth()
    };

    let proxy_connector = ProxyHttpConnector::new(upstream_rx);

    let https = HttpsConnectorBuilder::new()
        .with_tls_config(rustls_config)
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .wrap_connector(proxy_connector);

    Ok(Client::builder(TokioExecutor::new())
        .http1_title_case_headers(true)
        .http1_preserve_header_case(true)
        .build(https))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_cert_files() -> (NamedTempFile, NamedTempFile) {
        // 자체 서명 테스트 인증서 생성
        let subject_alt_names = vec!["localhost".to_string()];
        let cert_params =
            rcgen::CertificateParams::new(subject_alt_names).expect("Failed to create params");
        let key_pair = rcgen::KeyPair::generate().expect("Failed to generate key pair");
        let cert = cert_params
            .self_signed(&key_pair)
            .expect("Failed to self-sign");

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        let mut cert_file = NamedTempFile::new().expect("Failed to create temp cert file");
        cert_file
            .write_all(cert_pem.as_bytes())
            .expect("Failed to write cert");

        let mut key_file = NamedTempFile::new().expect("Failed to create temp key file");
        key_file
            .write_all(key_pem.as_bytes())
            .expect("Failed to write key");

        (cert_file, key_file)
    }

    #[test]
    fn test_load_certs() {
        let (cert_file, _key_file) = create_test_cert_files();
        let certs = load_certs(cert_file.path().to_str().unwrap()).expect("Failed to load certs");
        assert!(!certs.is_empty(), "Should load at least one certificate");
    }

    #[test]
    fn test_load_private_key() {
        let (_cert_file, key_file) = create_test_cert_files();
        let key = load_private_key(key_file.path().to_str().unwrap());
        assert!(key.is_ok(), "Should successfully load private key");
    }

    #[test]
    fn test_load_certs_nonexistent_file() {
        let result = load_certs("/nonexistent/path/cert.pem");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_private_key_nonexistent_file() {
        let result = load_private_key("/nonexistent/path/key.pem");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_certs_invalid_content() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"not a valid cert").unwrap();
        let result = load_certs(file.path().to_str().unwrap());
        assert!(result.is_err(), "Should reject non-PEM non-DER content");
    }

    #[test]
    fn test_validate_client_cert_config_disabled() {
        let config = ClientCertConfig {
            cert_path: "/nonexistent".to_string(),
            key_path: "/nonexistent".to_string(),
            enabled: false,
            domain_certs: vec![],
        };
        assert!(validate_client_cert_config(&config).is_ok());
    }

    #[test]
    fn test_validate_client_cert_config_missing_cert() {
        let config = ClientCertConfig {
            cert_path: "/nonexistent/cert.pem".to_string(),
            key_path: "/nonexistent/key.pem".to_string(),
            enabled: true,
            domain_certs: vec![],
        };
        let result = validate_client_cert_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("인증서 파일"));
    }

    #[test]
    fn test_validate_client_cert_config_valid() {
        let (cert_file, key_file) = create_test_cert_files();
        let config = ClientCertConfig {
            cert_path: cert_file.path().to_str().unwrap().to_string(),
            key_path: key_file.path().to_str().unwrap().to_string(),
            enabled: true,
            domain_certs: vec![],
        };
        assert!(validate_client_cert_config(&config).is_ok());
    }

    #[test]
    fn test_load_private_key_invalid_content() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"not a valid key").unwrap();
        let result = load_private_key(file.path().to_str().unwrap());
        assert!(result.is_err());
    }

    // --- parse_certificate_info 테스트 ---

    #[test]
    fn test_parse_certificate_info_basic() {
        let (cert_file, _key_file) = create_test_cert_files();
        let info = parse_certificate_info(cert_file.path().to_str().unwrap()).expect("파싱 실패");

        // rcgen 기본 파라미터: CN은 설정하지 않으면 None일 수 있음
        // SAN에 localhost가 있어야 함
        assert!(
            info.sans_dns.contains(&"localhost".to_string()),
            "SAN DNS에 localhost가 있어야 함: {:?}",
            info.sans_dns
        );
        assert_eq!(info.chain_length, 1);
        assert!(!info.fingerprint_sha256.is_empty());
        assert!(!info.serial_number.is_empty());
        assert!(!info.not_before.is_empty());
        assert!(!info.not_after.is_empty());
    }

    #[test]
    fn test_parse_certificate_info_fingerprint_format() {
        let (cert_file, _key_file) = create_test_cert_files();
        let info = parse_certificate_info(cert_file.path().to_str().unwrap()).expect("파싱 실패");

        // SHA-256 지문은 colon-separated hex 형식이어야 함
        let parts: Vec<&str> = info.fingerprint_sha256.split(':').collect();
        assert_eq!(parts.len(), 32, "SHA-256은 32바이트여야 함");
        for part in &parts {
            assert_eq!(part.len(), 2, "각 바이트는 2자리 hex여야 함");
            assert!(
                part.chars().all(|c| c.is_ascii_hexdigit()),
                "hex 문자여야 함: {}",
                part
            );
        }
    }

    #[test]
    fn test_parse_certificate_info_nonexistent_file() {
        let result = parse_certificate_info("/nonexistent/cert.pem");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_certificate_info_invalid_content() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"not a valid cert").unwrap();
        let result = parse_certificate_info(file.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_certificate_info_self_signed_is_not_ca() {
        let (cert_file, _key_file) = create_test_cert_files();
        let info = parse_certificate_info(cert_file.path().to_str().unwrap()).expect("파싱 실패");
        // rcgen 기본 자체 서명 인증서는 CA가 아님
        assert!(!info.is_ca);
    }

    #[test]
    fn test_parse_certificate_info_ca_cert() {
        // CA 인증서 생성
        let mut ca_params =
            rcgen::CertificateParams::new(Vec::<String>::new()).expect("Failed to create params");
        ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        ca_params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "Test CA");
        ca_params
            .distinguished_name
            .push(rcgen::DnType::OrganizationName, "Test Org");
        let key_pair = rcgen::KeyPair::generate().expect("Failed to generate key pair");
        let cert = ca_params
            .self_signed(&key_pair)
            .expect("Failed to self-sign");

        let mut cert_file = NamedTempFile::new().unwrap();
        cert_file.write_all(cert.pem().as_bytes()).unwrap();

        let info = parse_certificate_info(cert_file.path().to_str().unwrap()).expect("파싱 실패");
        assert!(info.is_ca, "CA 인증서여야 함");
        assert_eq!(info.subject_cn, Some("Test CA".to_string()));
        assert_eq!(info.organization, Some("Test Org".to_string()));
    }

    // --- build_certified_key 테스트 ---

    #[test]
    fn test_build_certified_key_success() {
        let (cert_file, key_file) = create_test_cert_files();
        let result = build_certified_key(
            cert_file.path().to_str().unwrap(),
            key_file.path().to_str().unwrap(),
        );
        assert!(result.is_ok(), "CertifiedKey 빌드 성공해야 함");
    }

    #[test]
    fn test_build_certified_key_invalid_cert() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"not a cert").unwrap();
        let (_, key_file) = create_test_cert_files();
        let result = build_certified_key(
            file.path().to_str().unwrap(),
            key_file.path().to_str().unwrap(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_build_certified_key_mismatched_key() {
        // 두 개의 다른 키쌍 생성
        let (cert_file, _) = create_test_cert_files();
        let (_, other_key_file) = create_test_cert_files();
        let result = build_certified_key(
            cert_file.path().to_str().unwrap(),
            other_key_file.path().to_str().unwrap(),
        );
        // 키 불일치는 빌드 시점에서 에러가 날 수도, 안 날 수도 있음 (rustls 구현에 따라)
        // 최소한 패닉하지 않는 것만 확인
        let _ = result;
    }

    // --- DomainCertResolver 테스트 ---

    #[test]
    fn test_domain_cert_resolver_with_default() {
        let (cert_file, key_file) = create_test_cert_files();
        let key = build_certified_key(
            cert_file.path().to_str().unwrap(),
            key_file.path().to_str().unwrap(),
        )
        .unwrap();
        let resolver = DomainCertResolver {
            default_cert: Some(Arc::new(key)),
            domain_certs: vec![],
        };
        assert!(resolver.has_certs());
        assert!(resolver.resolve(&[], &[]).is_some());
    }

    #[test]
    fn test_domain_cert_resolver_without_default() {
        let resolver = DomainCertResolver {
            default_cert: None,
            domain_certs: vec![],
        };
        assert!(!resolver.has_certs());
        assert!(resolver.resolve(&[], &[]).is_none());
    }

    #[test]
    fn test_domain_cert_resolver_with_domain_certs_only() {
        let (cert_file, key_file) = create_test_cert_files();
        let key = build_certified_key(
            cert_file.path().to_str().unwrap(),
            key_file.path().to_str().unwrap(),
        )
        .unwrap();
        let resolver = DomainCertResolver {
            default_cert: None,
            domain_certs: vec![("*.example.com".to_string(), Arc::new(key))],
        };
        assert!(resolver.has_certs());
        // 기본 인증서가 없으면 None 반환 (현재 구현)
        assert!(resolver.resolve(&[], &[]).is_none());
    }
}
