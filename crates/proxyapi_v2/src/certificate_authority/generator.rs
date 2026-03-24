use std::fs;
#[cfg(feature = "openssl-ca")]
use std::path::PathBuf;
use tracing::{info, warn};

use super::storage::get_ca_storage_dir;

/// CA 인증서 만료 상태
enum CaExpiryStatus {
    /// 유효
    Valid,
    /// 곧 만료 (30일 미만, 남은 일수)
    ExpiringSoon(i64),
    /// 이미 만료됨
    Expired,
}

/// DER 바이트에서 인증서 만료 상태를 확인합니다.
fn check_cert_expiry(der_bytes: &[u8]) -> Option<CaExpiryStatus> {
    let (_, cert) = x509_parser::parse_x509_certificate(der_bytes).ok()?;
    let not_after = cert.validity().not_after.timestamp();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let remaining_days = (not_after - now) / 86400;
    Some(if remaining_days < 0 {
        CaExpiryStatus::Expired
    } else if remaining_days < 30 {
        CaExpiryStatus::ExpiringSoon(remaining_days)
    } else {
        CaExpiryStatus::Valid
    })
}

#[cfg(feature = "rcgen-ca")]
use super::RcgenAuthority;

#[cfg(feature = "openssl-ca")]
use super::OpensslAuthority;

#[cfg(feature = "rcgen-ca")]
use tokio_rustls::rustls::pki_types::CertificateDer;

/// 저장된 인증서를 로드하거나 새로 생성합니다.
#[cfg(feature = "rcgen-ca")]
pub fn load_or_generate_ca() -> Result<RcgenAuthority, String> {
    let storage_dir = get_ca_storage_dir()?;
    let key_path = storage_dir.join("cheolsu-proxy.key");
    let cer_path = storage_dir.join("cheolsu-proxy.cer");

    // 기존 인증서가 있으면 로드 (만료 확인 포함)
    if key_path.exists() && cer_path.exists() {
        if let Ok(cert_der) = fs::read(&cer_path) {
            match check_cert_expiry(&cert_der) {
                Some(CaExpiryStatus::Expired) => {
                    info!(path = %storage_dir.display(), "CA 인증서가 만료되었습니다. 새로 생성합니다.");
                    return generate_and_save_ca(&storage_dir);
                }
                Some(CaExpiryStatus::ExpiringSoon(days)) => {
                    warn!(
                        remaining_days = days,
                        "CA 인증서가 곧 만료됩니다. 재생성을 권장합니다."
                    );
                }
                _ => {}
            }
        }
        info!(path = %storage_dir.display(), "기존 CA 인증서 로드 중");
        return load_ca_from_storage(&key_path, &cer_path);
    }

    // 없으면 새로 생성
    info!(path = %storage_dir.display(), "새 CA 인증서 생성 중");
    generate_and_save_ca(&storage_dir)
}

/// 저장된 인증서 파일에서 RcgenAuthority를 로드합니다.
#[cfg(feature = "rcgen-ca")]
pub fn load_ca_from_storage(
    key_path: &std::path::Path,
    cer_path: &std::path::Path,
) -> Result<RcgenAuthority, String> {
    let key_pem =
        fs::read_to_string(key_path).map_err(|e| format!("Failed to read key file: {}", e))?;
    let cert_der =
        fs::read(cer_path).map_err(|e| format!("Failed to read certificate file: {}", e))?;

    let key_pair =
        rcgen::KeyPair::from_pem(&key_pem).map_err(|e| format!("Failed to parse key: {}", e))?;

    let cert_der = CertificateDer::from(cert_der);
    let ca_cert_params = rcgen::CertificateParams::from_ca_cert_der(&cert_der)
        .map_err(|e| format!("Failed to parse certificate: {}", e))?;

    let ca_cert = ca_cert_params
        .self_signed(&key_pair)
        .map_err(|e| format!("Failed to sign certificate: {}", e))?;

    Ok(RcgenAuthority::new(
        key_pair,
        ca_cert,
        1_000,
        tokio_rustls::rustls::crypto::aws_lc_rs::default_provider(),
    ))
}

/// 새로운 CA 인증서를 생성하고 저장합니다.
#[cfg(feature = "rcgen-ca")]
pub fn generate_and_save_ca(storage_dir: &std::path::Path) -> Result<RcgenAuthority, String> {
    // 키 생성
    let key_pair =
        rcgen::KeyPair::generate().map_err(|e| format!("Failed to generate key: {}", e))?;

    // CA 인증서 파라미터 설정
    let mut params = rcgen::CertificateParams::default();

    // CA 유효기간 명시 설정 (10년)
    use super::CA_TTL_SECS;
    use time::{Duration, OffsetDateTime};
    let not_before = OffsetDateTime::now_utc() - Duration::seconds(super::NOT_BEFORE_OFFSET);
    params.not_before = not_before;
    params.not_after = not_before + Duration::seconds(CA_TTL_SECS);

    // Distinguished Name 설정
    let mut distinguished_name = rcgen::DistinguishedName::new();
    distinguished_name.push(rcgen::DnType::CommonName, "Cheolsu Proxy Root CA");
    distinguished_name.push(rcgen::DnType::OrganizationName, "Cheolsu Proxy");
    distinguished_name.push(rcgen::DnType::CountryName, "KR");
    params.distinguished_name = distinguished_name;

    // CA 설정
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![
        rcgen::KeyUsagePurpose::KeyCertSign,
        rcgen::KeyUsagePurpose::CrlSign,
    ];

    // 자체 서명된 CA 인증서 생성
    let ca_cert = params
        .self_signed(&key_pair)
        .map_err(|e| format!("Failed to self-sign: {}", e))?;

    // 파일로 저장
    let key_path = storage_dir.join("cheolsu-proxy.key");
    let cer_path = storage_dir.join("cheolsu-proxy.cer");

    fs::write(&key_path, key_pair.serialize_pem())
        .map_err(|e| format!("Failed to save key: {}", e))?;
    fs::write(&cer_path, ca_cert.der())
        .map_err(|e| format!("Failed to save certificate (.cer): {}", e))?;

    // 권한 설정 (macOS/Linux)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set key permissions: {}", e))?;
    }

    info!(key = %key_path.display(), cert = %cer_path.display(), "CA 인증서 생성 완료");

    // 사용자에게 수동 설치 안내
    #[cfg(target_os = "macos")]
    {
        info!(
            cert_path = %cer_path.display(),
            folder_path = %cer_path.parent().unwrap_or(&cer_path).display(),
            "키체인에 수동으로 설치해주세요: \
            1) Keychain Access 앱 실행 \
            2) 'login' 키체인 선택 \
            3) File > Import Items... 선택 \
            4) 인증서 파일 선택 \
            5) 인증서를 더블클릭하고 '항상 신뢰'로 설정. \
            팁: Finder에서 Cmd+Shift+G로 폴더 열기"
        );
    }

    Ok(RcgenAuthority::new(
        key_pair,
        ca_cert,
        1_000,
        tokio_rustls::rustls::crypto::aws_lc_rs::default_provider(),
    ))
}

/// 커스텀 CA 인증서 파일이 존재하는지 확인합니다.
#[cfg(feature = "rcgen-ca")]
pub fn has_custom_ca() -> bool {
    if let Ok(storage_dir) = get_ca_storage_dir() {
        let cert_path = storage_dir.join("custom-ca.cer");
        let key_path = storage_dir.join("custom-ca.key");
        cert_path.exists() && key_path.exists()
    } else {
        false
    }
}

/// 커스텀 CA 인증서 파일 경로를 반환합니다.
#[cfg(feature = "rcgen-ca")]
pub fn get_custom_ca_paths() -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    let storage_dir = get_ca_storage_dir()?;
    Ok((
        storage_dir.join("custom-ca.cer"),
        storage_dir.join("custom-ca.key"),
    ))
}

/// CA 인증서를 빌드합니다.
/// 커스텀 CA가 존재하면 우선 사용하고, 없으면 자동 생성합니다.
#[cfg(feature = "rcgen-ca")]
pub fn build_ca() -> Result<RcgenAuthority, String> {
    if has_custom_ca() {
        let (cert_path, key_path) = get_custom_ca_paths()?;
        info!(cert = %cert_path.display(), "커스텀 CA 인증서 사용");
        load_ca_from_storage(&key_path, &cert_path)
    } else {
        info!("런타임 인증서 생성");
        load_or_generate_ca()
    }
}

/// OpenSSL Authority를 빌드합니다.
#[cfg(feature = "openssl-ca")]
pub fn build_openssl_ca() -> Result<OpensslAuthority, String> {
    info!("OpenSSL Authority 생성");
    load_or_generate_openssl_ca()
}

/// OpenSSL Authority를 로드하거나 생성합니다.
#[cfg(feature = "openssl-ca")]
pub fn load_or_generate_openssl_ca() -> Result<OpensslAuthority, String> {
    let storage_dir = get_ca_storage_dir()?;
    let key_path = storage_dir.join("cheolsu-proxy.key");
    let cer_path = storage_dir.join("cheolsu-proxy.cer");

    if key_path.exists() && cer_path.exists() {
        // PEM → DER 변환 후 만료 확인
        if let Ok(cert_pem_str) = fs::read_to_string(&cer_path) {
            if let Some(der_bytes) = pem::parse(cert_pem_str.as_bytes())
                .ok()
                .map(|p| p.contents().to_vec())
            {
                match check_cert_expiry(&der_bytes) {
                    Some(CaExpiryStatus::Expired) => {
                        info!(path = %storage_dir.display(), "OpenSSL CA 인증서가 만료되었습니다. 새로 생성합니다.");
                        return generate_openssl_ca(&storage_dir);
                    }
                    Some(CaExpiryStatus::ExpiringSoon(days)) => {
                        warn!(
                            remaining_days = days,
                            "OpenSSL CA 인증서가 곧 만료됩니다. 재생성을 권장합니다."
                        );
                    }
                    _ => {}
                }
            }
        }
        info!(path = %storage_dir.display(), "기존 OpenSSL 인증서 파일 사용");
        return load_openssl_ca_from_storage(&key_path, &cer_path);
    }

    info!("새로운 OpenSSL 인증서 생성");
    generate_openssl_ca(&storage_dir)
}

/// OpenSSL Authority를 스토리지에서 로드합니다.
#[cfg(feature = "openssl-ca")]
pub fn load_openssl_ca_from_storage(
    key_path: &PathBuf,
    cer_path: &PathBuf,
) -> Result<OpensslAuthority, String> {
    use openssl::hash::MessageDigest;
    use openssl::pkey::PKey;
    use openssl::x509::X509;
    let private_key_pem =
        fs::read_to_string(key_path).map_err(|e| format!("Failed to read private key: {}", e))?;

    // CA 인증서는 텍스트로 읽고 PEM로 파싱
    let ca_cert_pem = fs::read_to_string(cer_path)
        .map_err(|e| format!("Failed to read CA certificate: {}", e))?;

    info!(
        size_bytes = ca_cert_pem.len(),
        "CA 인증서 파일 로드 (PEM 형식)"
    );

    // PEM을 X509로 파싱
    let ca_cert = X509::from_pem(ca_cert_pem.as_bytes())
        .map_err(|e| format!("Failed to parse CA certificate from PEM: {}", e))?;

    // X509를 PEM으로 변환 (로깅용)
    let ca_cert_pem_converted = ca_cert
        .to_pem()
        .map_err(|e| format!("Failed to convert CA certificate to PEM: {}", e))?;

    info!(
        size_bytes = ca_cert_pem_converted.len(),
        "CA 인증서 PEM 파싱 성공"
    );

    // PEM을 PKey로 변환
    let pkey = PKey::private_key_from_pem(private_key_pem.as_bytes())
        .map_err(|e| format!("Failed to parse private key from PEM: {}", e))?;

    // OpensslAuthority 생성
    Ok(OpensslAuthority::new(
        pkey,
        ca_cert,
        MessageDigest::sha256(),
        1_000,
        tokio_rustls::rustls::crypto::aws_lc_rs::default_provider(),
    ))
}

/// 새로운 OpenSSL Authority를 생성합니다.
#[cfg(feature = "openssl-ca")]
pub fn generate_openssl_ca(storage_dir: &PathBuf) -> Result<OpensslAuthority, String> {
    use openssl::asn1::Asn1Time;
    use openssl::bn::BigNum;
    use openssl::hash::MessageDigest;
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;
    use openssl::x509::X509;
    use openssl::x509::X509Name;
    use openssl::x509::extension::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::CA_TTL_SECS;

    // RSA 키 페어 생성
    let rsa = Rsa::generate(2048).map_err(|e| format!("Failed to generate RSA key: {}", e))?;
    let pkey = PKey::from_rsa(rsa).map_err(|e| format!("Failed to create PKey from RSA: {}", e))?;

    // X509Name 생성
    let mut name =
        X509Name::builder().map_err(|e| format!("Failed to create X509Name builder: {}", e))?;
    name.append_entry_by_nid(openssl::nid::Nid::COUNTRYNAME, "KR")
        .map_err(|e| format!("Failed to set country: {}", e))?;
    name.append_entry_by_nid(openssl::nid::Nid::STATEORPROVINCENAME, "Seoul")
        .map_err(|e| format!("Failed to set state: {}", e))?;
    name.append_entry_by_nid(openssl::nid::Nid::ORGANIZATIONNAME, "Cheolsu Proxy")
        .map_err(|e| format!("Failed to set organization: {}", e))?;
    name.append_entry_by_nid(openssl::nid::Nid::ORGANIZATIONALUNITNAME, "Development")
        .map_err(|e| format!("Failed to set organizational unit: {}", e))?;
    name.append_entry_by_nid(openssl::nid::Nid::COMMONNAME, "Cheolsu Proxy CA")
        .map_err(|e| format!("Failed to set common name: {}", e))?;
    let name = name.build();

    // X509 인증서 빌더 생성
    let mut cert_builder =
        X509::builder().map_err(|e| format!("Failed to create X509 builder: {}", e))?;

    // 버전 설정 (v3)
    cert_builder
        .set_version(2)
        .map_err(|e| format!("Failed to set version: {}", e))?;

    // 시리얼 번호 설정
    let serial =
        BigNum::from_u32(1).map_err(|e| format!("Failed to create serial number: {}", e))?;
    let serial_integer = serial
        .to_asn1_integer()
        .map_err(|e| format!("Failed to convert serial to Asn1Integer: {}", e))?;
    cert_builder
        .set_serial_number(&serial_integer)
        .map_err(|e| format!("Failed to set serial number: {}", e))?;

    // 유효 기간 설정
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Failed to get current time: {}", e))?;
    let not_before = Asn1Time::from_unix(now.as_secs() as i64)
        .map_err(|e| format!("Failed to create not_before time: {}", e))?;
    let not_after = Asn1Time::from_unix(now.as_secs() as i64 + CA_TTL_SECS)
        .map_err(|e| format!("Failed to create not_after time: {}", e))?;

    cert_builder
        .set_not_before(&not_before)
        .map_err(|e| format!("Failed to set not_before: {}", e))?;
    cert_builder
        .set_not_after(&not_after)
        .map_err(|e| format!("Failed to set not_after: {}", e))?;

    // 주체와 발급자 설정
    cert_builder
        .set_subject_name(&name)
        .map_err(|e| format!("Failed to set subject name: {}", e))?;
    cert_builder
        .set_issuer_name(&name)
        .map_err(|e| format!("Failed to set issuer name: {}", e))?;

    // 공개키 설정
    cert_builder
        .set_pubkey(&pkey)
        .map_err(|e| format!("Failed to set public key: {}", e))?;

    // CA 확장 추가
    let basic_constraints = BasicConstraints::new()
        .critical()
        .ca()
        .pathlen(0)
        .build()
        .map_err(|e| format!("Failed to create basic constraints: {}", e))?;
    cert_builder
        .append_extension(basic_constraints)
        .map_err(|e| format!("Failed to add basic constraints: {}", e))?;

    let key_usage = KeyUsage::new()
        .critical()
        .key_cert_sign()
        .crl_sign()
        .build()
        .map_err(|e| format!("Failed to create key usage: {}", e))?;
    cert_builder
        .append_extension(key_usage)
        .map_err(|e| format!("Failed to add key usage: {}", e))?;

    // 인증서 서명
    cert_builder
        .sign(&pkey, MessageDigest::sha256())
        .map_err(|e| format!("Failed to sign certificate: {}", e))?;

    let ca_cert = cert_builder.build();

    // 파일로 저장
    fs::create_dir_all(storage_dir)
        .map_err(|e| format!("Failed to create storage directory: {}", e))?;

    let private_key_pem = pkey
        .private_key_to_pem_pkcs8()
        .map_err(|e| format!("Failed to convert private key to PEM: {}", e))?;
    let ca_cert_pem = ca_cert
        .to_pem()
        .map_err(|e| format!("Failed to convert certificate to PEM: {}", e))?;

    let key_path = storage_dir.join("cheolsu-proxy.key");
    fs::write(&key_path, &private_key_pem)
        .map_err(|e| format!("Failed to write private key: {}", e))?;
    fs::write(&storage_dir.join("cheolsu-proxy.cer"), &ca_cert_pem)
        .map_err(|e| format!("Failed to write CA certificate: {}", e))?;

    // 키 파일 권한 설정 (macOS/Linux)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set key permissions: {}", e))?;
    }

    info!(path = %storage_dir.display(), "OpenSSL CA 인증서 생성 완료");

    // OpensslAuthority 생성
    Ok(OpensslAuthority::new(
        pkey,
        ca_cert,
        MessageDigest::sha256(),
        1_000,
        tokio_rustls::rustls::crypto::aws_lc_rs::default_provider(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_cert_expiry_invalid_der() {
        assert!(check_cert_expiry(b"not a certificate").is_none());
    }

    #[test]
    fn test_check_cert_expiry_empty() {
        assert!(check_cert_expiry(b"").is_none());
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_check_cert_expiry_valid_cert() {
        let key_pair = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::default();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let cert = params.self_signed(&key_pair).unwrap();
        let der = cert.der().to_vec();

        assert!(matches!(
            check_cert_expiry(&der),
            Some(CaExpiryStatus::Valid)
        ));
    }

    #[test]
    fn test_ca_ttl_is_10_years() {
        use super::super::CA_TTL_SECS;
        assert_eq!(CA_TTL_SECS, 10 * 365 * 24 * 60 * 60);
    }

    #[test]
    fn test_leaf_ttl_is_1_year() {
        use super::super::LEAF_TTL_SECS;
        assert_eq!(LEAF_TTL_SECS, 365 * 24 * 60 * 60);
    }

    #[test]
    fn test_cache_ttl_is_half_leaf() {
        use super::super::{CACHE_TTL, LEAF_TTL_SECS};
        assert_eq!(CACHE_TTL, LEAF_TTL_SECS as u64 / 2);
    }
}
