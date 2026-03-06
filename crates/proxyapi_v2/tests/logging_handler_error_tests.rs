use proxyapi_v2::{
    Body,
    hyper::{Response, StatusCode},
};
use std::error::Error;
use tokio;

// LoggingHandler의 핵심 구조체들을 모킹
#[derive(Clone)]
pub struct MockProxiedRequest {
    pub method: String,
    pub uri: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Vec<u8>,
    pub timestamp: i64,
}

impl MockProxiedRequest {
    pub fn new(method: String, uri: String) -> Self {
        Self {
            method,
            uri,
            headers: std::collections::HashMap::new(),
            body: Vec::new(),
            timestamp: 1234567890,
        }
    }
}

#[derive(Clone)]
pub struct MockProxiedResponse {
    pub status: StatusCode,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Vec<u8>,
    pub timestamp: i64,
}

impl MockProxiedResponse {
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: std::collections::HashMap::new(),
            body: Vec::new(),
            timestamp: 1234567890,
        }
    }
}

// LoggingHandler의 핵심 기능을 모킹한 테스트용 핸들러
#[derive(Clone)]
pub struct TestLoggingHandler {
    pub req: Option<MockProxiedRequest>,
    pub res: Option<MockProxiedResponse>,
}

impl TestLoggingHandler {
    pub fn new() -> Self {
        Self {
            req: None,
            res: None,
        }
    }

    /// 캐시된 응답 데이터로부터 Response 생성
    fn create_response_from_cached_data(&self) -> Response<Body> {
        if let Some(cached_response) = &self.res {
            let mut response = Response::builder().status(cached_response.status);

            // 헤더 복사
            for (key, value) in &cached_response.headers {
                response = response.header(key, value);
            }

            // body 설정
            let body_data = String::from_utf8_lossy(&cached_response.body);
            response
                .body(Body::from(body_data.to_string()))
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from("Failed to create response from cached data"))
                        .unwrap()
                })
        } else {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("No cached response data available"))
                .unwrap()
        }
    }

    /// handle_error 메서드의 핵심 로직을 테스트
    pub async fn handle_error_test(&mut self, err: MockError) -> Response<Body> {
        eprintln!("❌ [HANDLER] handle_error 호출됨 - 에러 발생!");
        eprintln!("   - 에러 타입: {:?}", err);
        eprintln!("   - 에러 메시지: {}", err);

        // UnexpectedEof 에러인지 먼저 확인
        if let Some(source) = err.source() {
            let source_str = source.to_string();
            if source_str.contains("UnexpectedEof") || source_str.contains("unexpected EOF") {
                eprintln!("ℹ️  TLS close_notify 없이 연결 종료됨 - 정상 종료로 처리");

                // UnexpectedEof는 정상적인 연결 종료로 처리
                // 이미 받은 응답 데이터가 있는지 확인
                if self.res.is_some() {
                    eprintln!("   - ✅ 이미 받은 응답 데이터가 있음 - 해당 데이터 사용");
                    eprintln!("   - 📊 응답 상태: {}", self.res.as_ref().unwrap().status);
                    eprintln!(
                        "   - 📏 응답 크기: {} bytes",
                        self.res.as_ref().unwrap().body.len()
                    );
                    return self.create_response_from_cached_data();
                } else {
                    eprintln!("   - ⚠️  받은 응답 데이터가 없음 - 빈 응답 반환");
                    return Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::empty())
                        .unwrap_or_else(|_| {
                            Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from("Internal Server Error"))
                                .unwrap()
                        });
                }
            }
        }

        // 상세한 에러 정보 로깅 (UnexpectedEof가 아닌 경우만)
        eprintln!("❌ 프록시 요청 오류 발생:");
        eprintln!("   - 에러 타입: {:?}", err);
        eprintln!("   - 에러 메시지: {}", err);

        // 에러 원인 분석 및 curl 백업 사용 여부 결정
        let should_use_curl = if let Some(source) = err.source() {
            eprintln!("   - 원인: {}", source);

            let source_str = source.to_string();
            if source_str.contains("HandshakeFailure") {
                eprintln!("   - TLS 핸드셰이크 실패 (curl 백업 사용)");
                true
            } else {
                eprintln!("   - 기타 연결 오류 (curl 백업 사용 안함)");
                false
            }
        } else {
            eprintln!("   - 알 수 없는 오류 (curl 백업 사용 안함)");
            false
        };

        // TLS 오류인 경우 curl 백업 사용 (테스트에서는 모킹)
        if should_use_curl {
            if let Some(_req) = &self.req {
                eprintln!("🔄 TLS 오류: curl로 직접 요청 시도 중...");
                // 테스트에서는 curl 백업을 모킹
                return Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from("Curl fallback response"))
                    .unwrap();
            }
        }

        // curl도 실패한 경우 기본 에러 응답
        Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::from(format!("Proxy Error: {}", err)))
            .expect("Failed to build error response")
    }
}

// 테스트용 에러 타입들
#[derive(Debug, Clone)]
pub enum MockError {
    UnexpectedEof,
    HandshakeFailure,
    ConnectionRefused,
    Timeout,
    Unknown,
}

impl std::fmt::Display for MockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MockError::UnexpectedEof => write!(f, "UnexpectedEof"),
            MockError::HandshakeFailure => write!(f, "HandshakeFailure"),
            MockError::ConnectionRefused => write!(f, "ConnectionRefused"),
            MockError::Timeout => write!(f, "Timeout"),
            MockError::Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::error::Error for MockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MockError::UnexpectedEof => Some(&UnexpectedEofSource),
            MockError::HandshakeFailure => Some(&HandshakeFailureSource),
            MockError::ConnectionRefused => Some(&ConnectionRefusedSource),
            MockError::Timeout => Some(&TimeoutSource),
            MockError::Unknown => None,
        }
    }
}

// 에러 소스들
#[derive(Debug)]
struct UnexpectedEofSource;
impl std::fmt::Display for UnexpectedEofSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unexpected EOF")
    }
}
impl std::error::Error for UnexpectedEofSource {}

#[derive(Debug)]
struct HandshakeFailureSource;
impl std::fmt::Display for HandshakeFailureSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HandshakeFailure")
    }
}
impl std::error::Error for HandshakeFailureSource {}

#[derive(Debug)]
struct ConnectionRefusedSource;
impl std::fmt::Display for ConnectionRefusedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConnectionRefused")
    }
}
impl std::error::Error for ConnectionRefusedSource {}

#[derive(Debug)]
struct TimeoutSource;
impl std::fmt::Display for TimeoutSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Timeout")
    }
}
impl std::error::Error for TimeoutSource {}

#[tokio::test]
async fn test_handle_error_unexpected_eof_with_cached_response() {
    let mut handler = TestLoggingHandler::new();

    // 캐시된 응답 데이터 설정
    let mut cached_response = MockProxiedResponse::new(StatusCode::OK);
    cached_response.body = b"Hello, World!".to_vec();
    handler.res = Some(cached_response);

    // UnexpectedEof 에러 생성
    let error = MockError::UnexpectedEof;

    // handle_error 호출
    let response = handler.handle_error_test(error).await;

    // 검증
    assert_eq!(response.status(), StatusCode::OK);

    // 응답 본문 확인
    use http_body_util::BodyExt;
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body_bytes[..], b"Hello, World!");
}

#[tokio::test]
async fn test_handle_error_unexpected_eof_without_cached_response() {
    let mut handler = TestLoggingHandler::new();

    // 캐시된 응답 데이터 없음
    handler.res = None;

    // UnexpectedEof 에러 생성
    let error = MockError::UnexpectedEof;

    // handle_error 호출
    let response = handler.handle_error_test(error).await;

    // 검증
    assert_eq!(response.status(), StatusCode::OK);

    // 응답 본문이 비어있는지 확인
    use http_body_util::BodyExt;
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert!(body_bytes.is_empty());
}

#[tokio::test]
async fn test_handle_error_handshake_failure_with_request() {
    let mut handler = TestLoggingHandler::new();

    // 요청 데이터 설정
    let request = MockProxiedRequest::new("GET".to_string(), "https://example.com".to_string());
    handler.req = Some(request);

    // HandshakeFailure 에러 생성
    let error = MockError::HandshakeFailure;

    // handle_error 호출
    let response = handler.handle_error_test(error).await;

    // 검증 - curl 백업이 사용되어야 함
    assert_eq!(response.status(), StatusCode::OK);

    // 응답 본문 확인
    use http_body_util::BodyExt;
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body_bytes[..], b"Curl fallback response");
}

#[tokio::test]
async fn test_handle_error_handshake_failure_without_request() {
    let mut handler = TestLoggingHandler::new();

    // 요청 데이터 없음
    handler.req = None;

    // HandshakeFailure 에러 생성
    let error = MockError::HandshakeFailure;

    // handle_error 호출
    let response = handler.handle_error_test(error).await;

    // 검증 - curl 백업을 사용할 수 없으므로 BAD_GATEWAY 반환
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

    // 응답 본문 확인
    use http_body_util::BodyExt;
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert!(body_bytes.starts_with(b"Proxy Error:"));
}

#[tokio::test]
async fn test_handle_error_connection_refused() {
    let mut handler = TestLoggingHandler::new();

    // ConnectionRefused 에러 생성
    let error = MockError::ConnectionRefused;

    // handle_error 호출
    let response = handler.handle_error_test(error).await;

    // 검증 - curl 백업을 사용하지 않으므로 BAD_GATEWAY 반환
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

    // 응답 본문 확인
    use http_body_util::BodyExt;
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert!(body_bytes.starts_with(b"Proxy Error:"));
}

#[tokio::test]
async fn test_handle_error_timeout() {
    let mut handler = TestLoggingHandler::new();

    // Timeout 에러 생성
    let error = MockError::Timeout;

    // handle_error 호출
    let response = handler.handle_error_test(error).await;

    // 검증 - curl 백업을 사용하지 않으므로 BAD_GATEWAY 반환
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

    // 응답 본문 확인
    use http_body_util::BodyExt;
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert!(body_bytes.starts_with(b"Proxy Error:"));
}

#[tokio::test]
async fn test_handle_error_unknown() {
    let mut handler = TestLoggingHandler::new();

    // Unknown 에러 생성 (source가 None)
    let error = MockError::Unknown;

    // handle_error 호출
    let response = handler.handle_error_test(error).await;

    // 검증 - curl 백업을 사용하지 않으므로 BAD_GATEWAY 반환
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

    // 응답 본문 확인
    use http_body_util::BodyExt;
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert!(body_bytes.starts_with(b"Proxy Error:"));
}

#[tokio::test]
async fn test_create_response_from_cached_data() {
    let handler = TestLoggingHandler::new();

    // 캐시된 응답 데이터 설정
    let mut cached_response = MockProxiedResponse::new(StatusCode::CREATED);
    cached_response.body = b"Created resource".to_vec();
    cached_response
        .headers
        .insert("Content-Type".to_string(), "application/json".to_string());

    let mut test_handler = TestLoggingHandler::new();
    test_handler.res = Some(cached_response);

    // create_response_from_cached_data 호출
    let response = test_handler.create_response_from_cached_data();

    // 검증
    assert_eq!(response.status(), StatusCode::CREATED);

    // 응답 본문 확인
    use http_body_util::BodyExt;
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body_bytes[..], b"Created resource");
}

#[tokio::test]
async fn test_create_response_from_cached_data_no_data() {
    let mut handler = TestLoggingHandler::new();

    // 캐시된 응답 데이터 없음
    handler.res = None;

    // create_response_from_cached_data 호출
    let response = handler.create_response_from_cached_data();

    // 검증
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // 응답 본문 확인
    use http_body_util::BodyExt;
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body_bytes[..], b"No cached response data available");
}
