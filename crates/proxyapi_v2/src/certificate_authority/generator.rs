use std::fs;
#[cfg(feature = "openssl-ca")]
use std::path::PathBuf;
use tracing::{info, warn};

use super::storage::get_ca_storage_dir;

/// CA 인증서 만료 상태
#[derive(Debug, PartialEq)]
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
    let (ca, _event) = load_or_generate_ca_with_event()?;
    Ok(ca)
}

/// 저장된 인증서를 로드하거나 새로 생성하며, 발생한 이벤트를 함께 반환합니다.
#[cfg(feature = "rcgen-ca")]
pub fn load_or_generate_ca_with_event() -> Result<(RcgenAuthority, super::CertEvent), String> {
    let storage_dir = get_ca_storage_dir()?;
    let key_path = storage_dir.join("cheolsu-proxy.key");
    let cer_path = storage_dir.join("cheolsu-proxy.cer");

    // 기존 인증서가 있으면 로드 (만료 확인 포함)
    if key_path.exists() && cer_path.exists() {
        if let Ok(cert_der) = fs::read(&cer_path) {
            match check_cert_expiry(&cert_der) {
                Some(CaExpiryStatus::Expired) => {
                    info!(path = %storage_dir.display(), "CA 인증서가 만료되었습니다. 새로 생성합니다.");
                    let ca = generate_and_save_ca(&storage_dir)?;
                    return Ok((ca, super::CertEvent::CaRegeneratedExpired));
                }
                Some(CaExpiryStatus::ExpiringSoon(days)) => {
                    warn!(
                        remaining_days = days,
                        "CA 인증서가 {}일 후 만료. 자동 재생성합니다.", days
                    );
                    let ca = generate_and_save_ca(&storage_dir)?;
                    return Ok((ca, super::CertEvent::CaRegeneratedExpiringSoon(days)));
                }
                _ => {}
            }
        }
        info!(path = %storage_dir.display(), "기존 CA 인증서 로드 중");
        match load_ca_from_storage(&key_path, &cer_path) {
            Ok(ca) => return Ok((ca, super::CertEvent::CaLoaded)),
            Err(e) => {
                warn!(
                    "CA 인증서 로드 실패 (손상 또는 키 불일치): {}. 재생성합니다.",
                    e
                );
                let ca = generate_and_save_ca(&storage_dir)?;
                return Ok((ca, super::CertEvent::CaRegeneratedCorrupted(e)));
            }
        }
    }

    // 없으면 새로 생성
    info!(path = %storage_dir.display(), "새 CA 인증서 생성 중");
    let ca = generate_and_save_ca(&storage_dir)?;
    Ok((ca, super::CertEvent::CaGenerated))
}

/// CA 인증서와 개인키가 매칭되는지 검증하고, 검증된 KeyPair를 반환합니다.
#[cfg(feature = "rcgen-ca")]
fn verify_ca_key_cert_match(key_pem: &str, cert_der: &[u8]) -> Result<rcgen::KeyPair, String> {
    use x509_parser::parse_x509_certificate;

    let key_pair =
        rcgen::KeyPair::from_pem(key_pem).map_err(|e| format!("개인키 파싱 실패: {}", e))?;
    let (_, cert) =
        parse_x509_certificate(cert_der).map_err(|e| format!("인증서 파싱 실패: {}", e))?;

    // 인증서의 SPKI DER과 개인키의 SPKI DER을 비교
    let cert_spki = cert.public_key().raw;
    let key_spki_pem = key_pair.public_key_pem();
    let key_spki_der = pem::parse(key_spki_pem)
        .map_err(|e| format!("공개키 PEM 파싱 실패: {}", e))?
        .into_contents();

    if cert_spki != key_spki_der {
        return Err("CA 인증서와 개인키가 매칭되지 않습니다. 인증서를 재생성합니다.".to_string());
    }

    Ok(key_pair)
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

    let key_pair = verify_ca_key_cert_match(&key_pem, &cert_der)?;

    let cert_der = CertificateDer::from(cert_der);
    let ca_cert_pem = pem::encode(&pem::Pem::new("CERTIFICATE", cert_der.to_vec()));
    let issuer = rcgen::Issuer::from_ca_cert_der(&cert_der, key_pair)
        .map_err(|e| format!("Failed to parse CA certificate for issuer: {}", e))?;

    Ok(RcgenAuthority::new(
        issuer,
        cert_der,
        ca_cert_pem,
        key_pem,
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

    let key_pem = key_pair.serialize_pem();
    fs::write(&key_path, &key_pem).map_err(|e| format!("Failed to save key: {}", e))?;
    let ca_cert_der_bytes = ca_cert.der().to_vec();
    fs::write(&cer_path, &ca_cert_der_bytes)
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

    let ca_cert_pem = pem::encode(&pem::Pem::new("CERTIFICATE", ca_cert_der_bytes));
    let ca_cert_der: CertificateDer<'static> = ca_cert.into();
    let issuer = rcgen::Issuer::from_ca_cert_der(&ca_cert_der, key_pair)
        .map_err(|e| format!("Failed to create issuer: {}", e))?;

    Ok(RcgenAuthority::new(
        issuer,
        ca_cert_der,
        ca_cert_pem,
        key_pem,
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
                            "OpenSSL CA 인증서가 {}일 후 만료. 자동 재생성합니다.", days
                        );
                        return generate_openssl_ca(&storage_dir);
                    }
                    _ => {}
                }
            }
        }
        info!(path = %storage_dir.display(), "기존 OpenSSL 인증서 파일 사용");
        match load_openssl_ca_from_storage(&key_path, &cer_path) {
            Ok(ca) => return Ok(ca),
            Err(e) => {
                warn!(
                    "OpenSSL CA 로드 실패 (손상 또는 키 불일치): {}. 재생성합니다.",
                    e
                );
                return generate_openssl_ca(&storage_dir);
            }
        }
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

    let ca_cert = X509::from_pem(ca_cert_pem.as_bytes())
        .map_err(|e| format!("Failed to parse CA certificate from PEM: {}", e))?;

    let pkey = PKey::private_key_from_pem(private_key_pem.as_bytes())
        .map_err(|e| format!("Failed to parse private key from PEM: {}", e))?;
    let cert_pub_key_der = ca_cert
        .public_key()
        .map_err(|e| format!("인증서에서 공개키 추출 실패: {}", e))?
        .public_key_to_der()
        .map_err(|e| format!("공개키 DER 변환 실패: {}", e))?;
    let pkey_pub_key_der = pkey
        .public_key_to_der()
        .map_err(|e| format!("개인키에서 공개키 DER 변환 실패: {}", e))?;
    if cert_pub_key_der != pkey_pub_key_der {
        return Err("OpenSSL CA 인증서와 개인키가 매칭되지 않습니다".to_string());
    }

    info!("CA 인증서 PEM 파싱 및 키 매칭 검증 성공");

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

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_check_cert_expiry_expiring_soon() {
        use time::{Duration, OffsetDateTime};

        let key_pair = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::default();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        // 10일 후 만료되도록 설정
        let now = OffsetDateTime::now_utc();
        params.not_before = now - Duration::days(350);
        params.not_after = now + Duration::days(10);
        let cert = params.self_signed(&key_pair).unwrap();
        let der = cert.der().to_vec();

        match check_cert_expiry(&der) {
            Some(CaExpiryStatus::ExpiringSoon(days)) => {
                assert!(days >= 9 && days <= 10, "남은 일수: {}", days);
            }
            other => panic!("ExpiringSoon이어야 하지만 {:?}", other),
        }
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_check_cert_expiry_expired() {
        use time::{Duration, OffsetDateTime};

        let key_pair = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::default();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        // 이미 만료된 인증서
        let now = OffsetDateTime::now_utc();
        params.not_before = now - Duration::days(400);
        params.not_after = now - Duration::days(1);
        let cert = params.self_signed(&key_pair).unwrap();
        let der = cert.der().to_vec();

        assert!(matches!(
            check_cert_expiry(&der),
            Some(CaExpiryStatus::Expired)
        ));
    }

    #[test]
    fn test_ca_ttl_is_10_years() {
        use super::super::CA_TTL_SECS;
        assert_eq!(CA_TTL_SECS, 10 * 365 * 24 * 60 * 60);
    }

    #[test]
    fn test_leaf_ttl_is_90_days() {
        use super::super::LEAF_TTL_SECS;
        assert_eq!(LEAF_TTL_SECS, 90 * 24 * 60 * 60);
    }

    #[test]
    fn test_cache_ttl_is_half_leaf() {
        use super::super::{CACHE_TTL, LEAF_TTL_SECS};
        assert_eq!(CACHE_TTL, LEAF_TTL_SECS as u64 / 2);
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_verify_ca_key_cert_match_valid() {
        let key_pair = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::default();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let cert = params.self_signed(&key_pair).unwrap();
        let key_pem = key_pair.serialize_pem();
        let cert_der = cert.der().to_vec();

        assert!(verify_ca_key_cert_match(&key_pem, &cert_der).is_ok());
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_verify_ca_key_cert_match_mismatch() {
        // 키 A로 만든 인증서에 키 B를 검증하면 실패해야 함
        let key_a = rcgen::KeyPair::generate().unwrap();
        let key_b = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::default();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let cert = params.self_signed(&key_a).unwrap();
        let key_b_pem = key_b.serialize_pem();
        let cert_der = cert.der().to_vec();

        assert!(verify_ca_key_cert_match(&key_b_pem, &cert_der).is_err());
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_verify_ca_key_cert_match_invalid_key_pem() {
        let key_pair = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::default();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let cert = params.self_signed(&key_pair).unwrap();
        let cert_der = cert.der().to_vec();

        assert!(verify_ca_key_cert_match("invalid pem", &cert_der).is_err());
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_verify_ca_key_cert_match_invalid_cert_der() {
        let key_pair = rcgen::KeyPair::generate().unwrap();
        let key_pem = key_pair.serialize_pem();

        assert!(verify_ca_key_cert_match(&key_pem, b"invalid der").is_err());
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_load_or_generate_ca_with_event_returns_event() {
        let result = load_or_generate_ca_with_event();
        assert!(result.is_ok());
        let (_ca, event) = result.unwrap();
        // 처음 실행이면 CaGenerated 또는 CaLoaded
        assert!(
            matches!(
                event,
                super::super::CertEvent::CaGenerated
                    | super::super::CertEvent::CaLoaded
                    | super::super::CertEvent::CaRegeneratedExpiringSoon(_)
            ),
            "예상치 못한 이벤트: {:?}",
            event
        );
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_generate_and_save_ca_creates_files() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "cheolsu_ca_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&tmp_dir).unwrap();

        let result = generate_and_save_ca(&tmp_dir);
        assert!(result.is_ok());

        let key_path = tmp_dir.join("cheolsu-proxy.key");
        let cer_path = tmp_dir.join("cheolsu-proxy.cer");
        assert!(key_path.exists(), "키 파일이 생성되어야 함");
        assert!(cer_path.exists(), "인증서 파일이 생성되어야 함");

        // 생성된 인증서가 Valid인지 확인
        let cert_der = fs::read(&cer_path).unwrap();
        assert!(matches!(
            check_cert_expiry(&cert_der),
            Some(CaExpiryStatus::Valid)
        ));

        // 키-인증서 매칭 확인
        let key_pem = fs::read_to_string(&key_path).unwrap();
        assert!(verify_ca_key_cert_match(&key_pem, &cert_der).is_ok());

        // cleanup
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_verify_ca_key_cert_match_rsa_key() {
        let key_pair = rcgen::KeyPair::generate_for(&rcgen::PKCS_RSA_SHA256).unwrap();
        let mut params = rcgen::CertificateParams::default();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let cert = params.self_signed(&key_pair).unwrap();
        let key_pem = key_pair.serialize_pem();
        let cert_der = cert.der().to_vec();

        assert!(
            verify_ca_key_cert_match(&key_pem, &cert_der).is_ok(),
            "RSA 키 타입에서도 SPKI 비교가 정확해야 함"
        );
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_verify_ca_key_cert_match_ecdsa_p384_key() {
        let key_pair = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P384_SHA384).unwrap();
        let mut params = rcgen::CertificateParams::default();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let cert = params.self_signed(&key_pair).unwrap();
        let key_pem = key_pair.serialize_pem();
        let cert_der = cert.der().to_vec();

        assert!(
            verify_ca_key_cert_match(&key_pem, &cert_der).is_ok(),
            "ECDSA P-384 키 타입에서도 SPKI 비교가 정확해야 함"
        );
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_verify_ca_key_cert_match_ed25519_key() {
        let key_pair = rcgen::KeyPair::generate_for(&rcgen::PKCS_ED25519).unwrap();
        let mut params = rcgen::CertificateParams::default();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let cert = params.self_signed(&key_pair).unwrap();
        let key_pem = key_pair.serialize_pem();
        let cert_der = cert.der().to_vec();

        assert!(
            verify_ca_key_cert_match(&key_pem, &cert_der).is_ok(),
            "Ed25519 키 타입에서도 SPKI 비교가 정확해야 함"
        );
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_verify_ca_key_cert_match_returns_usable_keypair() {
        let key_pair = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::default();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let cert = params.self_signed(&key_pair).unwrap();
        let key_pem = key_pair.serialize_pem();
        let cert_der = cert.der().to_vec();

        // 반환된 KeyPair로 Issuer를 생성할 수 있어야 함
        let returned_kp = verify_ca_key_cert_match(&key_pem, &cert_der).unwrap();
        let cert_der_owned = tokio_rustls::rustls::pki_types::CertificateDer::from(cert_der);
        let issuer = rcgen::Issuer::from_ca_cert_der(&cert_der_owned, returned_kp);
        assert!(issuer.is_ok(), "반환된 KeyPair로 Issuer 생성이 가능해야 함");
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_generate_and_load_ca_roundtrip() {
        use super::super::CertificateAuthority;
        let tmp_dir = std::env::temp_dir().join(format!(
            "cheolsu_ca_roundtrip_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&tmp_dir).unwrap();

        // 생성
        let ca1 = generate_and_save_ca(&tmp_dir).unwrap();
        let ca1_der = ca1.get_ca_cert_der().unwrap();

        // 로드
        let key_path = tmp_dir.join("cheolsu-proxy.key");
        let cer_path = tmp_dir.join("cheolsu-proxy.cer");
        let ca2 = load_ca_from_storage(&key_path, &cer_path).unwrap();
        let ca2_der = ca2.get_ca_cert_der().unwrap();

        // 동일한 CA 인증서인지 확인
        assert_eq!(ca1_der, ca2_der, "생성 후 로드한 CA가 동일해야 함");

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_corrupted_cert_file_triggers_regeneration() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "cheolsu_ca_corrupt_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&tmp_dir).unwrap();

        // 정상 CA 생성
        generate_and_save_ca(&tmp_dir).unwrap();

        let cer_path = tmp_dir.join("cheolsu-proxy.cer");
        let key_path = tmp_dir.join("cheolsu-proxy.key");

        // 인증서 파일을 손상시킴
        fs::write(&cer_path, b"corrupted data").unwrap();

        // 손상된 인증서 로드 시 에러가 발생해야 함
        let result = load_ca_from_storage(&key_path, &cer_path);
        assert!(result.is_err(), "손상된 인증서 로드는 실패해야 함");

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_mismatched_key_cert_triggers_error() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "cheolsu_ca_mismatch_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&tmp_dir).unwrap();

        // CA 생성
        generate_and_save_ca(&tmp_dir).unwrap();

        // 다른 키로 덮어쓰기
        let other_key = rcgen::KeyPair::generate().unwrap();
        let key_path = tmp_dir.join("cheolsu-proxy.key");
        fs::write(&key_path, other_key.serialize_pem()).unwrap();

        // 키-인증서 불일치로 로드 실패
        let cer_path = tmp_dir.join("cheolsu-proxy.cer");
        let result = load_ca_from_storage(&key_path, &cer_path);
        assert!(result.is_err(), "키-인증서 불일치 시 로드가 실패해야 함");

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[cfg(feature = "rcgen-ca")]
    #[test]
    fn test_not_before_offset_applied() {
        use super::super::NOT_BEFORE_OFFSET;
        use time::OffsetDateTime;

        let key_pair = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::default();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);

        let now = OffsetDateTime::now_utc();
        params.not_before = now - time::Duration::seconds(NOT_BEFORE_OFFSET);
        params.not_after = now + time::Duration::days(90);
        let cert = params.self_signed(&key_pair).unwrap();
        let der = cert.der().to_vec();

        let (_, parsed) = x509_parser::parse_x509_certificate(&der).unwrap();
        let not_before_ts = parsed.validity().not_before.timestamp();
        let now_ts = now.unix_timestamp();

        // not_before가 현재 시간보다 NOT_BEFORE_OFFSET(2일) 이전이어야 함
        assert!(
            now_ts - not_before_ts >= NOT_BEFORE_OFFSET - 5,
            "not_before가 약 2일 전이어야 함"
        );
    }
}
