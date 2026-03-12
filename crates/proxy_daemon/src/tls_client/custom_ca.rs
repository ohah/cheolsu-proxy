use tracing::info;

use crate::protocol::CertificateInfo;

use super::loader::parse_x509_to_certificate_info;

/// PKCS12 (.p12/.pfx) 파일에서 인증서(PEM)와 개인키(PEM)를 추출합니다.
pub fn parse_pkcs12(
    p12_path: &str,
    password: &str,
) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
    let p12_data = std::fs::read(p12_path)?;
    let pkcs12 = openssl::pkcs12::Pkcs12::from_der(&p12_data)?;
    let parsed = pkcs12.parse2(password)?;

    let cert = parsed.cert.ok_or("PKCS12 파일에 인증서가 없습니다")?;
    let pkey = parsed.pkey.ok_or("PKCS12 파일에 개인키가 없습니다")?;

    let cert_pem = cert.to_pem()?;
    // PKCS8 형식으로 내보내기 (rcgen과 호환)
    let key_pem = pkey.private_key_to_pem_pkcs8()?;

    Ok((cert_pem, key_pem))
}

/// 커스텀 CA 인증서를 앱 데이터 디렉토리에 저장합니다.
/// cert_pem과 key_pem 바이트를 custom-ca.cer, custom-ca.key로 저장합니다.
pub fn save_custom_ca(cert_pem: &[u8], key_pem: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let storage_dir = proxyapi_v2::certificate_authority::get_ca_storage_dir()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let cert_path = storage_dir.join("custom-ca.cer");
    let key_path = storage_dir.join("custom-ca.key");

    // 인증서가 PEM 형식인지 확인하고, PEM이면 DER로 변환하여 저장
    // (기존 load_ca_from_storage는 cert를 DER, key를 PEM으로 읽음)
    let cert_data = if cert_pem.starts_with(b"-----BEGIN") {
        // PEM → DER 변환
        let x509 = openssl::x509::X509::from_pem(cert_pem)?;
        x509.to_der()?
    } else {
        cert_pem.to_vec()
    };

    std::fs::write(&cert_path, &cert_data)?;
    std::fs::write(&key_path, key_pem)?;

    // 키 파일 권한 제한 (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;
    }

    info!(
        cert = %cert_path.display(),
        key = %key_path.display(),
        "커스텀 CA 인증서 저장 완료"
    );

    Ok(())
}

/// 커스텀 CA 인증서를 제거합니다.
pub fn remove_custom_ca() -> Result<(), Box<dyn std::error::Error>> {
    let storage_dir = proxyapi_v2::certificate_authority::get_ca_storage_dir()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let cert_path = storage_dir.join("custom-ca.cer");
    let key_path = storage_dir.join("custom-ca.key");

    if cert_path.exists() {
        std::fs::remove_file(&cert_path)?;
    }
    if key_path.exists() {
        std::fs::remove_file(&key_path)?;
    }

    info!("커스텀 CA 인증서 제거 완료");
    Ok(())
}

/// 현재 커스텀 CA가 활성화되어 있는지 확인하고 인증서 정보를 반환합니다.
pub fn get_custom_ca_info() -> Result<Option<CertificateInfo>, Box<dyn std::error::Error>> {
    if !proxyapi_v2::certificate_authority::has_custom_ca() {
        return Ok(None);
    }

    let (cert_path, _) = proxyapi_v2::certificate_authority::get_custom_ca_paths()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    // DER 형식으로 저장되어 있으므로 DER에서 정보 파싱
    let cert_data = std::fs::read(&cert_path)?;
    let (_, cert) = x509_parser::parse_x509_certificate(&cert_data)
        .map_err(|e| format!("인증서 파싱 실패: {}", e))?;

    let info = parse_x509_to_certificate_info(&cert, &cert_data, 1);
    Ok(Some(info))
}
