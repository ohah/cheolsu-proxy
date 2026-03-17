use crate::tls_version_detector::TlsVersion;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 도메인별 학습된 TLS 전략 저장소
///
/// TLS 핸드셰이크 실패 시 반대 전략을 기록하여,
/// 다음 연결부터 자동으로 올바른 전략을 선택합니다.
pub type TlsStrategyCache = Arc<RwLock<HashMap<String, TlsStrategy>>>;

/// 학습된 TLS 전략 정보 (프론트엔드 전달용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedTlsStrategy {
    /// 도메인
    pub domain: String,
    /// 학습된 전략
    pub strategy: TlsStrategy,
}

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TlsStrategy {
    /// OpenSSL 전용 (TLS 1.0/1.1, SSL 3.0, 특수 도메인/암호화)
    OpenSslOnly,
    /// rustls 전용 (TLS 1.2/1.3 일반 연결)
    RustlsOnly,
}

impl TlsStrategy {
    /// 반대 전략을 반환합니다 (폴백용)
    pub fn opposite(self) -> Self {
        match self {
            TlsStrategy::OpenSslOnly => TlsStrategy::RustlsOnly,
            TlsStrategy::RustlsOnly => TlsStrategy::OpenSslOnly,
        }
    }
}

impl std::fmt::Display for TlsStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsStrategy::OpenSslOnly => write!(f, "OpenSSL"),
            TlsStrategy::RustlsOnly => write!(f, "rustls"),
        }
    }
}
