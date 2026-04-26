use serde::{Deserialize, Serialize};

/// 인증서 파일에서 추출한 상세 정보
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CertificateInfo {
    /// Subject CN
    pub subject_cn: Option<String>,
    /// Issuer CN
    pub issuer_cn: Option<String>,
    /// 조직명
    pub organization: Option<String>,
    /// DNS SAN 목록
    pub sans_dns: Vec<String>,
    /// IP SAN 목록
    pub sans_ip: Vec<String>,
    /// 유효기간 시작 (ISO 8601)
    pub not_before: String,
    /// 유효기간 끝 (ISO 8601)
    pub not_after: String,
    /// 시리얼 넘버 (hex)
    pub serial_number: String,
    /// SHA-256 지문 (hex, colon-separated)
    pub fingerprint_sha256: String,
    /// CA 인증서 여부
    pub is_ca: bool,
    /// 인증서 체인 길이
    pub chain_length: usize,
}

/// 프록시가 클라이언트에게 인증서를 요청하는 설정
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestClientCertConfig {
    /// 활성화 여부
    pub enabled: bool,
    /// 클라이언트 인증서를 검증할 CA 인증서 경로 (None이면 모든 인증서 수락)
    #[serde(default)]
    pub ca_cert_path: Option<String>,
    /// 인증서 필수 여부 (false면 선택적 요청 - 인증서 없어도 연결 허용)
    #[serde(default)]
    pub required: bool,
}

/// 도메인별 클라이언트 인증서 설정
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DomainClientCertConfig {
    /// 도메인 패턴 (예: "*.example.com", "api.service.io")
    pub domain_pattern: String,
    /// 클라이언트 인증서 파일 경로
    pub cert_path: String,
    /// 클라이언트 키 파일 경로
    pub key_path: String,
    /// 활성화 여부
    pub enabled: bool,
}

/// 클라이언트 인증서 설정 (mTLS)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientCertConfig {
    /// 클라이언트 인증서 파일 경로 (.pem, .crt)
    pub cert_path: String,
    /// 클라이언트 키 파일 경로 (.pem, .key)
    pub key_path: String,
    /// 활성화 여부
    pub enabled: bool,
    /// 도메인별 인증서 설정 (선택사항)
    #[serde(default)]
    pub domain_certs: Vec<DomainClientCertConfig>,
}

/// SSL Proxying 모드
/// - Blacklist: 모든 도메인 인터셉트, 목록에 있는 도메인만 패스스루
/// - Whitelist: 모든 도메인 패스스루, 목록에 있는 도메인만 인터셉트
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SslProxyingMode {
    /// 블랙리스트 모드 (기본): 모든 도메인 인터셉트, 목록에 있는 도메인만 패스스루
    #[default]
    Blacklist,
    /// 화이트리스트 모드: 모든 도메인 패스스루, 목록에 있는 도메인만 인터셉트
    Whitelist,
}

/// SSL Proxying 엔트리
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SslProxyingEntry {
    /// 도메인 패턴 (예: "example.com", "*.example.com", "example.com:443")
    pub pattern: String,
    pub enabled: bool,
}

/// TLS Passthrough 바이패스 항목
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TlsPassthroughEntry {
    pub host: String,
    pub failure_count: u32,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub blocked_by_never_passthrough: bool,
    #[serde(default)]
    pub last_failure_unix_secs: u64,
    #[serde(default)]
    pub expires_at_unix_secs: Option<u64>,
}
