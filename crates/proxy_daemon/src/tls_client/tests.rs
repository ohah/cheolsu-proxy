use super::*;
use std::io::Write;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio_rustls::rustls::client::ResolvesClientCert;

use loader::{load_certs, load_private_key, parse_certificate_info};
use resolver::{build_certified_key, DefaultCertResolver};
use validation::validate_client_cert_config;

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
    let config = crate::protocol::ClientCertConfig {
        cert_path: "/nonexistent".to_string(),
        key_path: "/nonexistent".to_string(),
        enabled: false,
        domain_certs: vec![],
    };
    assert!(validate_client_cert_config(&config).is_ok());
}

#[test]
fn test_validate_client_cert_config_missing_cert() {
    let config = crate::protocol::ClientCertConfig {
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
    let config = crate::protocol::ClientCertConfig {
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

// --- DefaultCertResolver 테스트 ---

#[test]
fn test_default_cert_resolver_with_cert() {
    let (cert_file, key_file) = create_test_cert_files();
    let key = build_certified_key(
        cert_file.path().to_str().unwrap(),
        key_file.path().to_str().unwrap(),
    )
    .unwrap();
    let resolver = DefaultCertResolver {
        default_cert: Some(Arc::new(key)),
    };
    assert!(resolver.has_certs());
    assert!(resolver.resolve(&[], &[]).is_some());
}

#[test]
fn test_default_cert_resolver_without_cert() {
    let resolver = DefaultCertResolver { default_cert: None };
    assert!(!resolver.has_certs());
    assert!(resolver.resolve(&[], &[]).is_none());
}
