#[cfg(feature = "openssl-ca")]
mod openssl_authority;
#[cfg(feature = "rcgen-ca")]
mod rcgen_authority;

use http::uri::Authority;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::CertificateDer;

#[cfg(feature = "openssl-ca")]
use openssl::ssl::SslContext;

#[cfg(feature = "openssl-ca")]
pub use openssl_authority::*;
#[cfg(feature = "rcgen-ca")]
pub use rcgen_authority::*;

const TTL_SECS: i64 = 365 * 24 * 60 * 60;
const CACHE_TTL: u64 = TTL_SECS as u64 / 2;
const NOT_BEFORE_OFFSET: i64 = 60;

/// 앱 데이터 디렉토리 경로를 반환합니다.
///
/// # Returns
/// - macOS: `~/Library/Application Support/com.cheolsu-proxy/`
/// - Windows: `%APPDATA%/com.cheolsu-proxy/`
/// - Linux: `~/.config/com.cheolsu-proxy/` (향후 구현)
fn get_ca_storage_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").map_err(|_| "Could not find HOME environment variable")?;

        // 앱 identifier (고정값)
        let identifier = "com.cheolsu-proxy";

        let dir = PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(identifier);

        // 디렉토리 생성
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create directory: {}", e))?;

        Ok(dir)
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .map_err(|_| "Could not find LOCALAPPDATA environment variable")?;

        // 앱 identifier (고정값)
        let identifier = "com.cheolsu-proxy";

        let dir = PathBuf::from(local_app_data).join(identifier);

        // 디렉토리 생성
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create directory: {}", e))?;

        Ok(dir)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Currently only macOS and Windows are supported".to_string())
    }
}

/// 캐시 저장 디렉토리 경로를 반환합니다.
///
/// # Returns
/// - macOS: `~/Library/Caches/com.cheolsu-proxy/data/{session_hash}/`
/// - Windows: `%LOCALAPPDATA%/com.cheolsu-proxy/Cache/data/{session_hash}/`
/// - Linux: `~/.cache/com.cheolsu-proxy/data/{session_hash}/` (향후 구현)
pub fn get_cache_storage_dir(session_hash: &str) -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").map_err(|_| "Could not find HOME environment variable")?;

        // 앱 identifier (고정값)
        let identifier = "com.cheolsu-proxy";

        let dir = PathBuf::from(home)
            .join("Library")
            .join("Caches")
            .join(identifier)
            .join("data")
            .join(session_hash);

        // 디렉토리 생성
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create cache directory: {}", e))?;

        Ok(dir)
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .map_err(|_| "Could not find LOCALAPPDATA environment variable")?;

        // 앱 identifier (고정값)
        let identifier = "com.cheolsu-proxy";

        let dir = PathBuf::from(local_app_data)
            .join(identifier)
            .join("Cache")
            .join("data")
            .join(session_hash);

        // 디렉토리 생성
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create cache directory: {}", e))?;

        Ok(dir)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Currently only macOS and Windows are supported".to_string())
    }
}

/// 세션 해시를 생성합니다.
/// 타임스탬프 + UUID 일부를 사용하여 고유성 보장
pub fn generate_session_hash() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let uuid_part = uuid::Uuid::new_v4().to_string().replace('-', "");

    format!("{}_{}", timestamp, &uuid_part[..8])
}

/// 앱 종료 시 현재 세션의 캐시만 삭제
pub fn clean_cache_on_exit(session_hash: &str) -> Result<(), String> {
    let cache_dir = get_cache_storage_dir(session_hash)?;

    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to remove session cache directory: {}", e))?;
        println!("✅ 세션 캐시 정리 완료: {}", cache_dir.display());
    }

    Ok(())
}

/// 지정된 일수보다 오래된 캐시 폴더 삭제
pub fn clean_old_cache(days: u64) -> Result<(), String> {
    let base_cache_dir = get_base_cache_dir()?;

    if !base_cache_dir.exists() {
        return Ok(());
    }

    let cutoff_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        - (days * 24 * 60 * 60);

    let mut cleaned_count = 0;

    if let Ok(entries) = fs::read_dir(&base_cache_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(modified_secs) = modified.duration_since(std::time::UNIX_EPOCH) {
                        if modified_secs.as_secs() < cutoff_time {
                            if let Err(e) = fs::remove_dir_all(entry.path()) {
                                eprintln!(
                                    "⚠️ 오래된 캐시 삭제 실패: {} - {}",
                                    entry.path().display(),
                                    e
                                );
                            } else {
                                cleaned_count += 1;
                                println!("🗑️ 오래된 캐시 삭제: {}", entry.path().display());
                            }
                        }
                    }
                }
            }
        }
    }

    println!("✅ 오래된 캐시 정리 완료: {} 개 폴더 삭제", cleaned_count);
    Ok(())
}

/// 모든 캐시 데이터 삭제 (수동 정리용)
pub fn clean_all_cache() -> Result<(), String> {
    let base_cache_dir = get_base_cache_dir()?;

    if !base_cache_dir.exists() {
        println!(
            "ℹ️ 캐시 디렉토리가 존재하지 않습니다: {}",
            base_cache_dir.display()
        );
        return Ok(());
    }

    fs::remove_dir_all(&base_cache_dir)
        .map_err(|e| format!("Failed to remove all cache directories: {}", e))?;

    println!("✅ 모든 캐시 정리 완료: {}", base_cache_dir.display());
    Ok(())
}

/// 기본 캐시 디렉토리 경로를 반환합니다 (세션 해시 제외)
fn get_base_cache_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").map_err(|_| "Could not find HOME environment variable")?;
        let identifier = "com.cheolsu-proxy";

        Ok(PathBuf::from(home)
            .join("Library")
            .join("Caches")
            .join(identifier)
            .join("data"))
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .map_err(|_| "Could not find LOCALAPPDATA environment variable")?;
        let identifier = "com.cheolsu-proxy";

        Ok(PathBuf::from(local_app_data)
            .join(identifier)
            .join("Cache")
            .join("data"))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Currently only macOS and Windows are supported".to_string())
    }
}

/// 저장된 인증서를 로드하거나 새로 생성합니다.
#[cfg(feature = "rcgen-ca")]
fn load_or_generate_ca() -> Result<RcgenAuthority, String> {
    let storage_dir = get_ca_storage_dir()?;
    let key_path = storage_dir.join("cheolsu-proxy.key");
    let cer_path = storage_dir.join("cheolsu-proxy.cer");

    // 기존 인증서가 있으면 로드
    if key_path.exists() && cer_path.exists() {
        println!("📁 기존 CA 인증서 로드 중: {}", storage_dir.display());
        return load_ca_from_storage(&key_path, &cer_path);
    }

    // 없으면 새로 생성
    println!("🔐 새 CA 인증서 생성 중: {}", storage_dir.display());
    generate_and_save_ca(&storage_dir)
}

/// 저장된 인증서 파일에서 RcgenAuthority를 로드합니다.
#[cfg(feature = "rcgen-ca")]
fn load_ca_from_storage(
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
fn generate_and_save_ca(storage_dir: &std::path::Path) -> Result<RcgenAuthority, String> {
    // 키 생성
    let key_pair =
        rcgen::KeyPair::generate().map_err(|e| format!("Failed to generate key: {}", e))?;

    // CA 인증서 파라미터 설정
    let mut params = rcgen::CertificateParams::default();

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

    println!("✅ CA 인증서 생성 완료:");
    println!("   키: {}", key_path.display());
    println!("   인증서: {}", cer_path.display());

    // 사용자에게 수동 설치 안내
    #[cfg(target_os = "macos")]
    {
        println!("🔐 키체인에 수동으로 설치해주세요:");
        println!("   1. Keychain Access 앱을 실행하세요");
        println!("   2. 'login' 키체인을 선택하세요");
        println!("   3. File > Import Items... 메뉴를 선택하세요");
        println!("   4. 다음 파일을 선택하세요:");
        println!("      📁 경로: {}", cer_path.display());
        println!("   5. 인증서를 더블클릭하고 '항상 신뢰'로 설정하세요");
        println!("");
        println!("💡 팁: Finder에서 폴더 열기:");
        println!("   - ⌘+Shift+G를 누르고 다음 경로를 입력하세요:");
        println!("   {}", cer_path.parent().unwrap().display());
    }

    Ok(RcgenAuthority::new(
        key_pair,
        ca_cert,
        1_000,
        tokio_rustls::rustls::crypto::aws_lc_rs::default_provider(),
    ))
}

/// 개발/테스트용 인증서를 사용하여 RcgenAuthority 생성
#[cfg(feature = "rcgen-ca")]
fn build_ca_embedded() -> Result<RcgenAuthority, String> {
    println!("🔧 개발/테스트 모드: 기존 인증서 파일 사용");

    // 실제 생성된 인증서 파일이 있으면 사용, 없으면 고정 파일 사용
    let storage_dir = get_ca_storage_dir()?;
    let key_path = storage_dir.join("cheolsu-proxy.key");
    let cer_path = storage_dir.join("cheolsu-proxy.cer");

    if key_path.exists() && cer_path.exists() {
        println!("📁 기존 생성된 인증서 파일 사용: {}", storage_dir.display());
        return load_ca_from_storage(&key_path, &cer_path);
    }

    // 기존 인증서가 없으면 고정 파일 사용 (개발용)
    println!("📁 고정 인증서 파일 사용 (개발용)");
    let private_key_bytes: &[u8] =
        include_bytes!("../../src/certificate_authority/cheolsu-proxy.key");
    let ca_cert_bytes: &[u8] = include_bytes!("../../src/certificate_authority/cheolsu-proxy.cer");

    // PEM 형식의 키 페어 파싱
    let key_pair = rcgen::KeyPair::from_pem(
        std::str::from_utf8(private_key_bytes)
            .map_err(|e| format!("Key file encoding error: {}", e))?,
    )
    .map_err(|e| format!("Failed to parse key pair: {}", e))?;

    // PEM 형식의 CA 인증서 파싱
    let ca_cert_params = rcgen::CertificateParams::from_ca_cert_pem(
        std::str::from_utf8(ca_cert_bytes)
            .map_err(|e| format!("Certificate file encoding error: {}", e))?,
    )
    .map_err(|e| format!("Failed to parse CA certificate: {}", e))?;

    // CertificateParams를 Certificate로 변환
    let ca_cert = ca_cert_params
        .self_signed(&key_pair)
        .map_err(|e| format!("Failed to sign CA certificate: {}", e))?;

    // RcgenAuthority 생성
    let ca = RcgenAuthority::new(
        key_pair,
        ca_cert,
        1_000,
        tokio_rustls::rustls::crypto::aws_lc_rs::default_provider(),
    );

    Ok(ca)
}

/// CA 인증서를 빌드합니다.

#[cfg(feature = "rcgen-ca")]
pub fn build_ca() -> Result<RcgenAuthority, String> {
    println!("✅ 런타임 인증서 생성");
    load_or_generate_ca()
}

/// OpenSSL Authority를 빌드합니다.
#[cfg(feature = "openssl-ca")]
pub fn build_openssl_ca() -> Result<OpensslAuthority, String> {
    println!("✅ OpenSSL Authority 생성");
    load_or_generate_openssl_ca()
}

/// OpenSSL Authority를 로드하거나 생성합니다.
#[cfg(feature = "openssl-ca")]
fn load_or_generate_openssl_ca() -> Result<OpensslAuthority, String> {
    let storage_dir = get_ca_storage_dir()?;
    let key_path = storage_dir.join("cheolsu-proxy.key");
    let cer_path = storage_dir.join("cheolsu-proxy.cer");

    if key_path.exists() && cer_path.exists() {
        println!(
            "📁 기존 OpenSSL 인증서 파일 사용: {}",
            storage_dir.display()
        );
        return load_openssl_ca_from_storage(&key_path, &cer_path);
    }

    println!("🔧 새로운 OpenSSL 인증서 생성");
    generate_openssl_ca(&storage_dir)
}

/// OpenSSL Authority를 스토리지에서 로드합니다.
#[cfg(feature = "openssl-ca")]
fn load_openssl_ca_from_storage(
    key_path: &PathBuf,
    cer_path: &PathBuf,
) -> Result<OpensslAuthority, String> {
    use openssl::hash::MessageDigest;
    use openssl::pkey::PKey;
    use openssl::x509::X509;
    use tracing::info;
    let private_key_pem =
        fs::read_to_string(key_path).map_err(|e| format!("Failed to read private key: {}", e))?;

    // CA 인증서는 텍스트로 읽고 PEM로 파싱
    let ca_cert_pem = fs::read_to_string(cer_path)
        .map_err(|e| format!("Failed to read CA certificate: {}", e))?;

    info!(
        "🔧 CA 인증서 파일 로드: {} bytes (PEM 형식)",
        ca_cert_pem.len()
    );

    // PEM을 X509로 파싱
    let ca_cert = X509::from_pem(ca_cert_pem.as_bytes())
        .map_err(|e| format!("Failed to parse CA certificate from PEM: {}", e))?;

    // X509를 PEM으로 변환 (로깅용)
    let ca_cert_pem_converted = ca_cert
        .to_pem()
        .map_err(|e| format!("Failed to convert CA certificate to PEM: {}", e))?;

    info!(
        "✅ CA 인증서 PEM 파싱 성공: {} bytes",
        ca_cert_pem_converted.len()
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
fn generate_openssl_ca(storage_dir: &PathBuf) -> Result<OpensslAuthority, String> {
    use openssl::asn1::Asn1Time;
    use openssl::bn::BigNum;
    use openssl::hash::MessageDigest;
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;
    use openssl::x509::X509;
    use openssl::x509::X509Name;
    use openssl::x509::extension::*;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    let not_after = Asn1Time::from_unix(now.as_secs() as i64 + TTL_SECS)
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

    fs::write(&storage_dir.join("cheolsu-proxy.key"), &private_key_pem)
        .map_err(|e| format!("Failed to write private key: {}", e))?;
    fs::write(&storage_dir.join("cheolsu-proxy.cer"), &ca_cert_pem)
        .map_err(|e| format!("Failed to write CA certificate: {}", e))?;

    println!("✅ OpenSSL CA 인증서 생성 완료: {}", storage_dir.display());

    // OpensslAuthority 생성
    Ok(OpensslAuthority::new(
        pkey,
        ca_cert,
        MessageDigest::sha256(),
        1_000,
        tokio_rustls::rustls::crypto::aws_lc_rs::default_provider(),
    ))
}

/// Issues certificates for use when communicating with clients.
///
/// Clients should be configured to either trust the provided root certificate, or to ignore
/// certificate errors.
pub trait CertificateAuthority: Send + Sync + 'static {
    /// Generate ServerConfig for use with rustls.
    fn gen_server_config(
        &self,
        authority: &Authority,
    ) -> impl Future<Output = Arc<ServerConfig>> + Send;

    /// Get the CA certificate in DER format for adding to client trust store.
    /// Returns None if the CA certificate is not available in DER format.
    fn get_ca_cert_der(&self) -> Option<Vec<u8>>;

    #[cfg(feature = "openssl-ca")]
    /// Generate OpenSSL SslContext for use with openssl.
    fn gen_openssl_context(
        &self,
        authority: &Authority,
    ) -> impl Future<Output = Result<SslContext, Box<dyn std::error::Error + Send + Sync>>> + Send;
}
