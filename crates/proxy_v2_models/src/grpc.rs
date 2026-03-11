use http::HeaderMap;
use serde::{Deserialize, Serialize};

/// gRPC 메타데이터 (서비스/메서드/상태 등)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GrpcMetadata {
    pub service: Option<String>,
    pub method: Option<String>,
    pub status_code: Option<i32>,
    pub status_message: Option<String>,
    pub content_subtype: Option<String>,
    pub streaming_type: GrpcStreamingType,
    pub frame_count: usize,
    pub is_compressed: bool,
}

/// gRPC 스트리밍 타입
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum GrpcStreamingType {
    #[default]
    Unary,
    ServerStreaming,
    ClientStreaming,
    BidirectionalStreaming,
}

/// gRPC 프레임 (Length-Prefixed Message)
pub struct GrpcFrame {
    pub compressed: bool,
    pub data: Vec<u8>,
}

/// gRPC body에서 프레임들을 추출
/// 형식: [Compressed-Flag(1B)][Message-Length(4B BE)][Message]
pub fn parse_grpc_frames(body: &[u8]) -> Vec<GrpcFrame> {
    let mut frames = Vec::new();
    let mut offset = 0;

    while offset + 5 <= body.len() {
        let compressed = body[offset] != 0;
        let length = u32::from_be_bytes([
            body[offset + 1],
            body[offset + 2],
            body[offset + 3],
            body[offset + 4],
        ]) as usize;

        offset += 5;

        if offset + length > body.len() {
            break;
        }

        frames.push(GrpcFrame {
            compressed,
            data: body[offset..offset + length].to_vec(),
        });

        offset += length;
    }

    frames
}

/// URI path에서 서비스명/메서드명 추출
/// 형식: /package.ServiceName/MethodName
pub fn extract_grpc_service_method(path: &str) -> (Option<String>, Option<String>) {
    // path가 "/"로 시작하는 형태: /package.Service/Method
    let trimmed = path.trim_start_matches('/');
    let parts: Vec<&str> = trimmed.splitn(2, '/').collect();

    match parts.len() {
        2 => (Some(parts[0].to_string()), Some(parts[1].to_string())),
        1 if !parts[0].is_empty() => (Some(parts[0].to_string()), None),
        _ => (None, None),
    }
}

/// Content-Type에서 서브타입 추출
/// 예: "application/grpc+proto" → Some("proto"), "application/grpc" → None
pub fn extract_grpc_content_subtype(content_type: &str) -> Option<String> {
    let lower = content_type.to_lowercase();

    // "application/grpc-web+proto" 또는 "application/grpc+proto" 패턴
    if let Some(pos) = lower.find("grpc") {
        let after_grpc = &lower[pos + 4..];
        // "+subtype" 패턴 찾기 (grpc-web+proto 같은 경우도 처리)
        if let Some(plus_pos) = after_grpc.find('+') {
            let subtype = after_grpc[plus_pos + 1..].trim();
            if !subtype.is_empty() {
                return Some(subtype.to_string());
            }
        }
    }

    None
}

/// grpc-status 헤더에서 상태 정보 추출
pub fn extract_grpc_status(headers: &HeaderMap) -> (Option<i32>, Option<String>) {
    let status_code = headers
        .get("grpc-status")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<i32>().ok());

    let status_message = headers
        .get("grpc-message")
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            // gRPC 메시지는 percent-encoding될 수 있음
            percent_decode(s)
        });

    (status_code, status_message)
}

/// 간단한 percent-decoding (grpc-message용)
fn percent_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// gRPC 상태 코드 이름 매핑
pub fn grpc_status_name(code: i32) -> &'static str {
    match code {
        0 => "OK",
        1 => "CANCELLED",
        2 => "UNKNOWN",
        3 => "INVALID_ARGUMENT",
        4 => "DEADLINE_EXCEEDED",
        5 => "NOT_FOUND",
        6 => "ALREADY_EXISTS",
        7 => "PERMISSION_DENIED",
        8 => "RESOURCE_EXHAUSTED",
        9 => "FAILED_PRECONDITION",
        10 => "ABORTED",
        11 => "OUT_OF_RANGE",
        12 => "UNIMPLEMENTED",
        13 => "INTERNAL",
        14 => "UNAVAILABLE",
        15 => "DATA_LOSS",
        16 => "UNAUTHENTICATED",
        _ => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    #[test]
    fn test_parse_grpc_frames_single() {
        // Compressed=0, Length=3, Data=[0x08, 0x96, 0x01]
        let body = vec![0x00, 0x00, 0x00, 0x00, 0x03, 0x08, 0x96, 0x01];
        let frames = parse_grpc_frames(&body);
        assert_eq!(frames.len(), 1);
        assert!(!frames[0].compressed);
        assert_eq!(frames[0].data, vec![0x08, 0x96, 0x01]);
    }

    #[test]
    fn test_parse_grpc_frames_multiple() {
        let mut body = vec![];
        // Frame 1: uncompressed, 2 bytes
        body.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x02, 0xAA, 0xBB]);
        // Frame 2: compressed, 1 byte
        body.extend_from_slice(&[0x01, 0x00, 0x00, 0x00, 0x01, 0xCC]);
        let frames = parse_grpc_frames(&body);
        assert_eq!(frames.len(), 2);
        assert!(!frames[0].compressed);
        assert_eq!(frames[0].data, vec![0xAA, 0xBB]);
        assert!(frames[1].compressed);
        assert_eq!(frames[1].data, vec![0xCC]);
    }

    #[test]
    fn test_parse_grpc_frames_empty() {
        let frames = parse_grpc_frames(&[]);
        assert!(frames.is_empty());
    }

    #[test]
    fn test_parse_grpc_frames_truncated() {
        // Header says 10 bytes but only 3 available
        let body = vec![0x00, 0x00, 0x00, 0x00, 0x0A, 0x01, 0x02, 0x03];
        let frames = parse_grpc_frames(&body);
        assert!(frames.is_empty());
    }

    #[test]
    fn test_extract_grpc_service_method() {
        let (svc, method) = extract_grpc_service_method("/grpc.health.v1.Health/Check");
        assert_eq!(svc.unwrap(), "grpc.health.v1.Health");
        assert_eq!(method.unwrap(), "Check");
    }

    #[test]
    fn test_extract_grpc_service_method_no_method() {
        let (svc, method) = extract_grpc_service_method("/MyService");
        assert_eq!(svc.unwrap(), "MyService");
        assert!(method.is_none());
    }

    #[test]
    fn test_extract_grpc_service_method_empty() {
        let (svc, method) = extract_grpc_service_method("/");
        assert!(svc.is_none());
        assert!(method.is_none());
    }

    #[test]
    fn test_extract_grpc_content_subtype() {
        assert_eq!(
            extract_grpc_content_subtype("application/grpc+proto"),
            Some("proto".to_string())
        );
        assert_eq!(
            extract_grpc_content_subtype("application/grpc-web+json"),
            Some("json".to_string())
        );
        assert_eq!(extract_grpc_content_subtype("application/grpc"), None);
    }

    #[test]
    fn test_extract_grpc_status() {
        let mut headers = HeaderMap::new();
        headers.insert("grpc-status", HeaderValue::from_static("0"));
        headers.insert("grpc-message", HeaderValue::from_static("OK"));
        let (code, msg) = extract_grpc_status(&headers);
        assert_eq!(code, Some(0));
        assert_eq!(msg, Some("OK".to_string()));
    }

    #[test]
    fn test_extract_grpc_status_missing() {
        let headers = HeaderMap::new();
        let (code, msg) = extract_grpc_status(&headers);
        assert!(code.is_none());
        assert!(msg.is_none());
    }

    #[test]
    fn test_extract_grpc_status_percent_encoded() {
        let mut headers = HeaderMap::new();
        headers.insert("grpc-status", HeaderValue::from_static("3"));
        headers.insert(
            "grpc-message",
            HeaderValue::from_static("invalid%20argument"),
        );
        let (code, msg) = extract_grpc_status(&headers);
        assert_eq!(code, Some(3));
        assert_eq!(msg, Some("invalid argument".to_string()));
    }

    #[test]
    fn test_grpc_status_name() {
        assert_eq!(grpc_status_name(0), "OK");
        assert_eq!(grpc_status_name(1), "CANCELLED");
        assert_eq!(grpc_status_name(13), "INTERNAL");
        assert_eq!(grpc_status_name(16), "UNAUTHENTICATED");
        assert_eq!(grpc_status_name(99), "UNKNOWN");
    }
}
