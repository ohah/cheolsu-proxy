use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use proxyapi_v2::{
    upstream_proxy::{ProxyHttpConnector, UpstreamProxyConfig},
    Body,
};
use std::path::Path;
use tokio_rustls::rustls::{
    crypto::aws_lc_rs,
    pki_types::{CertificateDer, PrivateKeyDer},
    ClientConfig,
};
use tracing::{error, info};

use crate::protocol::ClientCertConfig;

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
            match (
                load_certs(&cert_config.cert_path),
                load_private_key(&cert_config.key_path),
            ) {
                (Ok(certs), Ok(key)) => {
                    info!(
                        "클라이언트 인증서 로드 성공: cert={}, key={}",
                        cert_config.cert_path, cert_config.key_path
                    );
                    config_builder.with_client_auth_cert(certs, key)?
                }
                (Err(e), _) => {
                    error!("클라이언트 인증서 로드 실패: {}", e);
                    config_builder.with_no_client_auth()
                }
                (_, Err(e)) => {
                    error!("클라이언트 키 로드 실패: {}", e);
                    config_builder.with_no_client_auth()
                }
            }
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
        };
        assert!(validate_client_cert_config(&config).is_ok());
    }

    #[test]
    fn test_validate_client_cert_config_missing_cert() {
        let config = ClientCertConfig {
            cert_path: "/nonexistent/cert.pem".to_string(),
            key_path: "/nonexistent/key.pem".to_string(),
            enabled: true,
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
}
