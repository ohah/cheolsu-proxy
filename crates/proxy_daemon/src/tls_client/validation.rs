use std::path::Path;

use crate::protocol::{CertificateInfo, ClientCertConfig};

use super::loader::{load_certs, load_private_key, parse_certificate_info};

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

/// 인증서 파일이 CA 인증서인지 검증합니다.
/// CA 인증서는 BasicConstraints CA=true가 필요합니다.
pub fn validate_ca_certificate(
    cert_path: &str,
) -> Result<CertificateInfo, Box<dyn std::error::Error>> {
    let info = parse_certificate_info(cert_path)?;

    if !info.is_ca {
        return Err("CA 인증서가 아닙니다. BasicConstraints CA=true가 필요합니다".into());
    }

    Ok(info)
}

/// 인증서 바이트가 CA 인증서인지 검증합니다.
pub fn validate_ca_certificate_from_bytes(
    cert_bytes: &[u8],
) -> Result<CertificateInfo, Box<dyn std::error::Error>> {
    let info = super::loader::parse_certificate_info_from_bytes(cert_bytes)?;

    if !info.is_ca {
        return Err("CA 인증서가 아닙니다. BasicConstraints CA=true가 필요합니다".into());
    }

    Ok(info)
}

/// 인증서와 키가 매칭되는 쌍인지 검증합니다.
pub fn validate_cert_key_pair(
    cert_pem: &[u8],
    key_pem: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let cert = openssl::x509::X509::from_pem(cert_pem)
        .or_else(|_| openssl::x509::X509::from_der(cert_pem))?;
    let pkey = openssl::pkey::PKey::private_key_from_pem(key_pem)?;

    if !cert.public_key()?.public_eq(&pkey) {
        return Err("인증서와 키가 매칭되지 않습니다. 올바른 키 파일을 선택하세요".into());
    }

    Ok(())
}
