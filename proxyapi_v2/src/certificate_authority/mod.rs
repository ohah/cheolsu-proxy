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
/// - Windows: `%APPDATA%/com.cheolsu-proxy/` (향후 구현)
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

    #[cfg(not(target_os = "macos"))]
    {
        Err("Currently only macOS is supported".to_string())
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

    /// Generate PKCS12 identity for use with native-tls (TLS 1.0/1.1 support).
    /// Returns None if PKCS12 generation is not supported.
    #[cfg(feature = "native-tls-client")]
    fn gen_pkcs12_identity(
        &self,
        authority: &Authority,
    ) -> impl Future<Output = Option<Vec<u8>>> + Send;
}
