use crate::tls_version_detector::TlsVersion;

/// TLS 연결 정보를 담는 구조체
#[derive(Debug, Clone)]
pub struct TlsConnectionInfo {
    /// TLS 버전
    pub version: TlsVersion,
    /// TLS 버전 코드 (예: [0x03, 0x03])
    pub version_code: [u8; 2],
    /// 암호화 스위트 목록
    pub cipher_suites: Vec<u16>,
    /// Extensions 정보
    pub extensions: Vec<TlsExtension>,
    /// SNI (Server Name Indication) 지원 여부
    pub has_sni: bool,
    /// Apple 특별 암호화 스위트 포함 여부
    pub has_apple_cipher: bool,
    /// ClientHello 메시지 크기
    pub message_size: usize,
    /// 연결 복잡도 점수 (높을수록 복잡한 연결)
    pub complexity_score: u8,
}

/// TLS Extension 정보
#[derive(Debug, Clone)]
pub struct TlsExtension {
    pub extension_type: u16,
    pub name: String,
    pub length: u16,
}

/// TLS 처리 전략 — 결정적(deterministic) 선택만 사용
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsStrategy {
    /// OpenSSL 전용 (TLS 1.0/1.1, SSL 3.0, 특수 도메인/암호화)
    OpenSslOnly,
    /// rustls 전용 (TLS 1.2/1.3 일반 연결)
    RustlsOnly,
}
