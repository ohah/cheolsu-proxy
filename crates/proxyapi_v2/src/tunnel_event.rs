use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri, Version};
use proxy_v2_models::RequestInfo;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 헤더맵에 값을 안전하게 삽입합니다. 파싱 실패 시 해당 헤더를 건너뜁니다.
fn insert_header(headers: &mut HeaderMap, name: HeaderName, value: &str) {
    if let Ok(v) = HeaderValue::from_str(value) {
        headers.insert(name, v);
    }
}

/// 터널 모드 이벤트 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TunnelEventType {
    /// 터널 모드 시작
    Started,
    /// 터널 모드 완료
    Completed,
    /// 터널 모드 오류
    Error,
}

/// 터널 모드 이벤트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelEvent {
    /// 이벤트 타입
    pub event_type: TunnelEventType,
    /// 대상 서버 주소
    pub target_addr: String,
    /// 클라이언트 주소
    pub client_addr: String,
    /// 클라이언트에서 서버로 전송된 바이트 수
    pub client_to_server_bytes: u64,
    /// 서버에서 클라이언트로 전송된 바이트 수
    pub server_to_client_bytes: u64,
    /// 소요 시간
    pub duration: Duration,
    /// 오류 메시지 (오류 발생 시에만)
    pub error_message: Option<String>,
    /// 타임스탬프
    pub timestamp: std::time::SystemTime,
}

impl TunnelEvent {
    /// 터널 시작 이벤트 생성
    pub fn started(target_addr: String, client_addr: String) -> Self {
        Self {
            event_type: TunnelEventType::Started,
            target_addr,
            client_addr,
            client_to_server_bytes: 0,
            server_to_client_bytes: 0,
            duration: Duration::ZERO,
            error_message: None,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// 터널 완료 이벤트 생성
    pub fn completed(
        target_addr: String,
        client_addr: String,
        client_to_server_bytes: u64,
        server_to_client_bytes: u64,
        duration: Duration,
    ) -> Self {
        Self {
            event_type: TunnelEventType::Completed,
            target_addr,
            client_addr,
            client_to_server_bytes,
            server_to_client_bytes,
            duration,
            error_message: None,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// 터널 오류 이벤트 생성
    pub fn error(
        target_addr: String,
        client_addr: String,
        error_message: String,
        duration: Duration,
    ) -> Self {
        Self {
            event_type: TunnelEventType::Error,
            target_addr,
            client_addr,
            client_to_server_bytes: 0,
            server_to_client_bytes: 0,
            duration,
            error_message: Some(error_message),
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// 터널 이벤트를 RequestInfo로 변환합니다.
    /// URI 파싱 실패 등으로 변환이 불가능한 경우 None을 반환합니다.
    pub fn to_request_info(&self) -> Option<RequestInfo> {
        let timestamp = self
            .timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        match self.event_type {
            TunnelEventType::Started => {
                // 터널 시작: CONNECT 요청으로 표현
                let uri: Uri = format!("tunnel://{}", self.target_addr).parse().ok()?;
                let mut headers = HeaderMap::new();
                insert_header(
                    &mut headers,
                    HeaderName::from_static("x-tunnel-mode"),
                    "true",
                );
                insert_header(
                    &mut headers,
                    HeaderName::from_static("x-target-addr"),
                    &self.target_addr,
                );
                insert_header(
                    &mut headers,
                    HeaderName::from_static("x-client-addr"),
                    &self.client_addr,
                );

                let request = proxy_v2_models::ProxiedRequest::new(
                    Method::CONNECT,
                    uri,
                    Version::HTTP_11,
                    headers,
                    Bytes::new(),
                    timestamp,
                );

                Some(RequestInfo {
                    request: Some(request.for_client(None)),
                    response: None,
                    validations: None,
                    server_cert: None,
                    tls_fallback_used: None,
                })
            }
            TunnelEventType::Completed => {
                // 터널 완료: 200 응답으로 표현
                let mut headers = HeaderMap::new();
                insert_header(
                    &mut headers,
                    HeaderName::from_static("x-tunnel-mode"),
                    "completed",
                );
                insert_header(
                    &mut headers,
                    HeaderName::from_static("x-target-addr"),
                    &self.target_addr,
                );
                insert_header(
                    &mut headers,
                    HeaderName::from_static("x-client-addr"),
                    &self.client_addr,
                );
                insert_header(
                    &mut headers,
                    HeaderName::from_static("x-client-to-server-bytes"),
                    &self.client_to_server_bytes.to_string(),
                );
                insert_header(
                    &mut headers,
                    HeaderName::from_static("x-server-to-client-bytes"),
                    &self.server_to_client_bytes.to_string(),
                );
                insert_header(
                    &mut headers,
                    HeaderName::from_static("x-duration-ms"),
                    &self.duration.as_millis().to_string(),
                );

                let response = proxy_v2_models::ProxiedResponse::new(
                    StatusCode::OK,
                    Version::HTTP_11,
                    headers,
                    Bytes::new(),
                    timestamp,
                );

                Some(RequestInfo {
                    request: None,
                    response: Some(response.for_client("tunnel_completed", None)),
                    validations: None,
                    server_cert: None,
                    tls_fallback_used: None,
                })
            }
            TunnelEventType::Error => {
                // 터널 오류: 500 응답으로 표현
                let mut headers = HeaderMap::new();
                insert_header(
                    &mut headers,
                    HeaderName::from_static("x-tunnel-mode"),
                    "error",
                );
                insert_header(
                    &mut headers,
                    HeaderName::from_static("x-target-addr"),
                    &self.target_addr,
                );
                insert_header(
                    &mut headers,
                    HeaderName::from_static("x-client-addr"),
                    &self.client_addr,
                );
                insert_header(
                    &mut headers,
                    HeaderName::from_static("x-duration-ms"),
                    &self.duration.as_millis().to_string(),
                );

                if let Some(ref error_msg) = self.error_message {
                    insert_header(
                        &mut headers,
                        HeaderName::from_static("x-error-message"),
                        error_msg,
                    );
                }

                let response = proxy_v2_models::ProxiedResponse::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Version::HTTP_11,
                    headers,
                    Bytes::new(),
                    timestamp,
                );

                Some(RequestInfo {
                    request: None,
                    response: Some(response.for_client("tunnel_error", None)),
                    validations: None,
                    server_cert: None,
                    tls_fallback_used: None,
                })
            }
        }
    }
}
