use std::io;
use tokio::io::{AsyncRead, AsyncReadExt};

/// TLS 버전을 감지하는 유틸리티
pub struct TlsVersionDetector;

impl TlsVersionDetector {
    /// TLS ClientHello에서 TLS 버전을 감지합니다
    pub fn detect_tls_version(buffer: &[u8]) -> Option<TlsVersion> {
        if buffer.len() < 5 {
            return None;
        }

        // TLS 레코드 헤더 확인
        if buffer[0] != 0x16 {
            return None; // Handshake가 아님
        }

        // TLS 버전 확인 (3-4번째 바이트)
        let version_bytes = [buffer[3], buffer[4]];
        match version_bytes {
            [0x03, 0x01] => Some(TlsVersion::Tls10),
            [0x03, 0x02] => Some(TlsVersion::Tls11),
            [0x03, 0x03] => Some(TlsVersion::Tls12),
            [0x03, 0x04] => Some(TlsVersion::Tls13),
            _ => None,
        }
    }

    /// 스트림에서 TLS 버전을 비동기적으로 감지합니다
    pub async fn detect_from_stream<R: AsyncRead + Unpin>(
        stream: &mut R,
    ) -> io::Result<Option<TlsVersion>> {
        let mut buffer = [0u8; 5];
        let bytes_read = stream.read(&mut buffer).await?;
        
        if bytes_read < 5 {
            return Ok(None);
        }

        Ok(Self::detect_tls_version(&buffer))
    }

    /// TLS 버전이 rustls에서 지원되는지 확인합니다
    pub fn is_rustls_supported(version: TlsVersion) -> bool {
        matches!(version, TlsVersion::Tls12 | TlsVersion::Tls13)
    }

    /// TLS 버전이 OpenSSL에서 지원되는지 확인합니다
    pub fn is_openssl_supported(version: TlsVersion) -> bool {
        matches!(
            version,
            TlsVersion::Tls10 | TlsVersion::Tls11 | TlsVersion::Tls12 | TlsVersion::Tls13
        )
    }
}

/// 지원되는 TLS 버전
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVersion {
    Tls10,
    Tls11,
    Tls12,
    Tls13,
}

impl TlsVersion {
    /// TLS 버전을 문자열로 반환합니다
    pub fn as_str(&self) -> &'static str {
        match self {
            TlsVersion::Tls10 => "TLS 1.0",
            TlsVersion::Tls11 => "TLS 1.1",
            TlsVersion::Tls12 => "TLS 1.2",
            TlsVersion::Tls13 => "TLS 1.3",
        }
    }

    /// TLS 버전을 바이트 배열로 반환합니다
    pub fn as_bytes(&self) -> [u8; 2] {
        match self {
            TlsVersion::Tls10 => [0x03, 0x01],
            TlsVersion::Tls11 => [0x03, 0x02],
            TlsVersion::Tls12 => [0x03, 0x03],
            TlsVersion::Tls13 => [0x03, 0x04],
        }
    }
}

impl std::fmt::Display for TlsVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_tls10() {
        let tls10_hello = [0x16, 0x03, 0x01, 0x03, 0x01, 0x00, 0x98];
        assert_eq!(
            TlsVersionDetector::detect_tls_version(&tls10_hello),
            Some(TlsVersion::Tls10)
        );
    }

    #[test]
    fn test_detect_tls11() {
        let tls11_hello = [0x16, 0x03, 0x01, 0x03, 0x02, 0x00, 0x98];
        assert_eq!(
            TlsVersionDetector::detect_tls_version(&tls11_hello),
            Some(TlsVersion::Tls11)
        );
    }

    #[test]
    fn test_detect_tls12() {
        let tls12_hello = [0x16, 0x03, 0x01, 0x03, 0x03, 0x00, 0x98];
        assert_eq!(
            TlsVersionDetector::detect_tls_version(&tls12_hello),
            Some(TlsVersion::Tls12)
        );
    }

    #[test]
    fn test_detect_tls13() {
        let tls13_hello = [0x16, 0x03, 0x01, 0x03, 0x04, 0x00, 0x98];
        assert_eq!(
            TlsVersionDetector::detect_tls_version(&tls13_hello),
            Some(TlsVersion::Tls13)
        );
    }

    #[test]
    fn test_rustls_support() {
        assert!(!TlsVersionDetector::is_rustls_supported(TlsVersion::Tls10));
        assert!(!TlsVersionDetector::is_rustls_supported(TlsVersion::Tls11));
        assert!(TlsVersionDetector::is_rustls_supported(TlsVersion::Tls12));
        assert!(TlsVersionDetector::is_rustls_supported(TlsVersion::Tls13));
    }

    #[test]
    fn test_openssl_support() {
        assert!(TlsVersionDetector::is_openssl_supported(TlsVersion::Tls10));
        assert!(TlsVersionDetector::is_openssl_supported(TlsVersion::Tls11));
        assert!(TlsVersionDetector::is_openssl_supported(TlsVersion::Tls12));
        assert!(TlsVersionDetector::is_openssl_supported(TlsVersion::Tls13));
    }
}
