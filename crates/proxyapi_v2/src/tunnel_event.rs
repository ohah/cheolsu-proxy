use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode, Version};
use proxy_v2_models::RequestInfo;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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

    /// 터널 이벤트를 RequestInfo로 변환합니다
    pub fn to_request_info(&self) -> RequestInfo {
        let timestamp = self
            .timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        match self.event_type {
            TunnelEventType::Started => {
                // 터널 시작: CONNECT 요청으로 표현
                let uri = format!("tunnel://{}", self.target_addr).parse().unwrap();
                let mut headers = HeaderMap::new();
                headers.insert("X-Tunnel-Mode", "true".parse().unwrap());
                headers.insert("X-Target-Addr", self.target_addr.parse().unwrap());
                headers.insert("X-Client-Addr", self.client_addr.parse().unwrap());

                let request = proxy_v2_models::ProxiedRequest::new(
                    Method::CONNECT,
                    uri,
                    Version::HTTP_11,
                    headers,
                    Bytes::new(),
                    timestamp,
                );

                RequestInfo(Some(request.for_client(None)), None)
            }
            TunnelEventType::Completed => {
                // 터널 완료: 200 응답으로 표현
                let mut headers = HeaderMap::new();
                headers.insert("X-Tunnel-Mode", "completed".parse().unwrap());
                headers.insert("X-Target-Addr", self.target_addr.parse().unwrap());
                headers.insert("X-Client-Addr", self.client_addr.parse().unwrap());
                headers.insert(
                    "X-Client-To-Server-Bytes",
                    self.client_to_server_bytes.to_string().parse().unwrap(),
                );
                headers.insert(
                    "X-Server-To-Client-Bytes",
                    self.server_to_client_bytes.to_string().parse().unwrap(),
                );
                headers.insert(
                    "X-Duration-Ms",
                    self.duration.as_millis().to_string().parse().unwrap(),
                );

                let response = proxy_v2_models::ProxiedResponse::new(
                    StatusCode::OK,
                    Version::HTTP_11,
                    headers,
                    Bytes::new(),
                    timestamp,
                );

                RequestInfo(None, Some(response.for_client("tunnel_completed", None)))
            }
            TunnelEventType::Error => {
                // 터널 오류: 500 응답으로 표현
                let mut headers = HeaderMap::new();
                headers.insert("X-Tunnel-Mode", "error".parse().unwrap());
                headers.insert("X-Target-Addr", self.target_addr.parse().unwrap());
                headers.insert("X-Client-Addr", self.client_addr.parse().unwrap());
                headers.insert(
                    "X-Duration-Ms",
                    self.duration.as_millis().to_string().parse().unwrap(),
                );

                if let Some(ref error_msg) = self.error_message {
                    headers.insert("X-Error-Message", error_msg.parse().unwrap());
                }

                let response = proxy_v2_models::ProxiedResponse::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Version::HTTP_11,
                    headers,
                    Bytes::new(),
                    timestamp,
                );

                RequestInfo(None, Some(response.for_client("tunnel_error", None)))
            }
        }
    }
}
