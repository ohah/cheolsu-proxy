use serde::{Deserialize, Serialize};

/// HTTP 요청/응답의 각 단계별 소요 시간 (밀리초)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TimingWaterfall {
    /// DNS 조회 시간
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_lookup_ms: Option<u64>,
    /// TCP 연결 시간
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tcp_connection_ms: Option<u64>,
    /// TLS 핸드셰이크 시간
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_handshake_ms: Option<u64>,
    /// Time to First Byte (요청 전송 ~ 첫 응답 바이트)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttfb_ms: Option<u64>,
    /// 컨텐츠 다운로드 시간
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_download_ms: Option<u64>,
    /// 전체 소요 시간
    pub total_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let timing = TimingWaterfall::default();
        assert_eq!(timing.total_ms, 0);
        assert!(timing.dns_lookup_ms.is_none());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let timing = TimingWaterfall {
            dns_lookup_ms: Some(5),
            tcp_connection_ms: Some(10),
            tls_handshake_ms: Some(15),
            ttfb_ms: Some(50),
            content_download_ms: Some(100),
            total_ms: 180,
        };
        let json = serde_json::to_string(&timing).unwrap();
        let parsed: TimingWaterfall = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, timing);
    }

    #[test]
    fn test_none_fields_not_serialized() {
        let timing = TimingWaterfall {
            total_ms: 100,
            ..Default::default()
        };
        let json = serde_json::to_string(&timing).unwrap();
        assert!(!json.contains("dns_lookup_ms"));
        assert!(json.contains("total_ms"));
    }
}
