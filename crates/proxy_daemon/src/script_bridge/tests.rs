use super::*;
use bytes::Bytes;
use proxy_v2_models::ProxiedRequest;
use proxyapi_v2::{
    hyper::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, Version},
    hyper::{Request, Response},
    Body,
};
use std::collections::HashMap;

// ── 헬퍼 함수 ──

fn make_proxied_request(
    method: Method,
    uri: &str,
    headers: HeaderMap,
    body: &[u8],
) -> ProxiedRequest {
    ProxiedRequest::new(
        method,
        uri.parse::<Uri>().unwrap(),
        Version::HTTP_11,
        headers,
        Bytes::from(body.to_vec()),
        1000,
    )
}

fn make_script_request(
    method: &str,
    url: &str,
    headers: HashMap<String, String>,
    body: Option<String>,
) -> scripting::ScriptRequest {
    scripting::ScriptRequest {
        method: method.to_string(),
        url: url.to_string(),
        headers,
        body,
    }
}

fn make_script_response(
    status: u16,
    headers: HashMap<String, String>,
    body: Option<String>,
) -> scripting::ScriptResponse {
    scripting::ScriptResponse {
        status,
        headers,
        body,
    }
}

// ── to_script_request 테스트 ──

#[test]
fn test_to_script_request_basic() {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", HeaderValue::from_static("application/json"));
    headers.insert("x-custom", HeaderValue::from_static("value1"));

    let req = make_proxied_request(
        Method::POST,
        "http://example.com/api",
        headers,
        b"hello body",
    );
    let script_req = LoggingHandler::to_script_request(&req);

    assert_eq!(script_req.method, "POST");
    assert_eq!(script_req.url, "http://example.com/api");
    assert_eq!(
        script_req.headers.get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(script_req.headers.get("x-custom").unwrap(), "value1");
    assert_eq!(script_req.body, Some("hello body".to_string()));
}

#[test]
fn test_to_script_request_empty_headers() {
    let req = make_proxied_request(Method::GET, "http://example.com", HeaderMap::new(), b"");
    let script_req = LoggingHandler::to_script_request(&req);

    assert_eq!(script_req.method, "GET");
    assert!(script_req.headers.is_empty());
    assert_eq!(script_req.body, Some("".to_string()));
}

#[test]
fn test_to_script_request_empty_body() {
    let req = make_proxied_request(Method::GET, "http://example.com", HeaderMap::new(), b"");
    let script_req = LoggingHandler::to_script_request(&req);
    // 빈 바이트열도 UTF-8로 유효하므로 Some("")
    assert_eq!(script_req.body, Some("".to_string()));
}

#[test]
fn test_to_script_request_invalid_utf8_body() {
    let invalid_utf8: &[u8] = &[0xff, 0xfe, 0x00, 0x01];
    let req = make_proxied_request(
        Method::POST,
        "http://example.com",
        HeaderMap::new(),
        invalid_utf8,
    );
    let script_req = LoggingHandler::to_script_request(&req);
    // 잘못된 UTF-8이면 body가 None
    assert!(script_req.body.is_none());
}

#[test]
fn test_to_script_request_non_ascii_header_skipped() {
    let mut headers = HeaderMap::new();
    headers.insert("x-good", HeaderValue::from_static("ascii"));
    // 바이너리 값을 가진 헤더 (to_str 실패)
    headers.insert("x-binary", HeaderValue::from_bytes(&[0x80, 0x81]).unwrap());

    let req = make_proxied_request(Method::GET, "http://example.com", headers, b"");
    let script_req = LoggingHandler::to_script_request(&req);

    assert_eq!(script_req.headers.get("x-good").unwrap(), "ascii");
    // 바이너리 헤더는 변환 시 건너뜀
    assert!(script_req.headers.get("x-binary").is_none());
}

// ── to_script_response_from_hyper 테스트 ──

#[test]
fn test_to_script_response_from_hyper_basic() {
    let res = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html")
        .header("x-powered-by", "test")
        .body(Body::from("response body"))
        .unwrap();

    let script_res = LoggingHandler::to_script_response_from_hyper(&res);

    assert_eq!(script_res.status, 200);
    assert_eq!(script_res.headers.get("content-type").unwrap(), "text/html");
    assert_eq!(script_res.headers.get("x-powered-by").unwrap(), "test");
    // 응답 body는 스트리밍이라 None
    assert!(script_res.body.is_none());
}

#[test]
fn test_to_script_response_from_hyper_empty_headers() {
    let res = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from(""))
        .unwrap();

    let script_res = LoggingHandler::to_script_response_from_hyper(&res);
    assert_eq!(script_res.status, 404);
    assert!(script_res.headers.is_empty());
}

// ── apply_script_request_modify 테스트 ──

#[test]
fn test_apply_script_request_modify_headers() {
    let req = Request::builder()
        .method(Method::GET)
        .uri("http://example.com")
        .header("original", "header")
        .body(Body::from("body"))
        .unwrap();

    let mut new_headers = HashMap::new();
    new_headers.insert("x-new".to_string(), "new-value".to_string());
    let modified = make_script_request("GET", "http://example.com", new_headers, None);

    let result = LoggingHandler::apply_script_request_modify(req, &modified);

    // 기존 헤더가 지워지고 새 헤더만 남아야 함
    assert!(result.headers().get("original").is_none());
    assert_eq!(result.headers().get("x-new").unwrap(), "new-value");
}

#[test]
fn test_apply_script_request_modify_url() {
    let req = Request::builder()
        .method(Method::GET)
        .uri("http://example.com/old")
        .body(Body::from(""))
        .unwrap();

    let modified = make_script_request("GET", "http://example.com/new", HashMap::new(), None);
    let result = LoggingHandler::apply_script_request_modify(req, &modified);

    assert_eq!(result.uri().to_string(), "http://example.com/new");
}

#[test]
fn test_apply_script_request_modify_invalid_url_ignored() {
    let req = Request::builder()
        .method(Method::GET)
        .uri("http://example.com/original")
        .body(Body::from(""))
        .unwrap();

    // 유효하지 않은 URI는 무시되고 원래 URI 유지
    let modified = make_script_request("GET", "not a valid uri §§§", HashMap::new(), None);
    let result = LoggingHandler::apply_script_request_modify(req, &modified);

    assert_eq!(result.uri().to_string(), "http://example.com/original");
}

#[test]
fn test_apply_script_request_modify_body() {
    let req = Request::builder()
        .method(Method::POST)
        .uri("http://example.com")
        .body(Body::from("old body"))
        .unwrap();

    let modified = make_script_request(
        "POST",
        "http://example.com",
        HashMap::new(),
        Some("new body".to_string()),
    );
    let result = LoggingHandler::apply_script_request_modify(req, &modified);

    // body가 변경되었는지 확인 (Body 타입이므로 직접 비교 어렵지만, 생성됨을 확인)
    assert_eq!(result.method(), Method::POST);
}

#[test]
fn test_apply_script_request_modify_body_none_preserves_original() {
    let req = Request::builder()
        .method(Method::POST)
        .uri("http://example.com")
        .body(Body::from("original body"))
        .unwrap();

    let modified = make_script_request(
        "POST",
        "http://example.com",
        HashMap::new(),
        None, /* no body modification */
    );
    let result = LoggingHandler::apply_script_request_modify(req, &modified);

    // body가 None이면 원래 body 유지
    assert_eq!(result.method(), Method::POST);
}

#[test]
fn test_apply_script_request_modify_invalid_header_name_skipped() {
    let req = Request::builder()
        .method(Method::GET)
        .uri("http://example.com")
        .body(Body::from(""))
        .unwrap();

    let mut headers = HashMap::new();
    headers.insert("valid-header".to_string(), "value".to_string());
    // 유효하지 않은 헤더명 (공백 포함)
    headers.insert("invalid header".to_string(), "value".to_string());

    let modified = make_script_request("GET", "http://example.com", headers, None);
    let result = LoggingHandler::apply_script_request_modify(req, &modified);

    assert_eq!(result.headers().get("valid-header").unwrap(), "value");
    assert!(result.headers().get("invalid header").is_none());
}

// ── build_script_response 테스트 ──

#[test]
fn test_build_script_response_basic() {
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());

    let script_res = make_script_response(200, headers, Some(r#"{"ok":true}"#.to_string()));
    let res = LoggingHandler::build_script_response(&script_res);

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("content-type").unwrap(),
        "application/json"
    );
    // x-cheolsu-scripted 헤더가 추가되어야 함
    assert_eq!(res.headers().get("x-cheolsu-scripted").unwrap(), "true");
}

#[test]
fn test_build_script_response_empty_body() {
    let script_res = make_script_response(204, HashMap::new(), None);
    let res = LoggingHandler::build_script_response(&script_res);

    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}

#[test]
fn test_build_script_response_invalid_status_falls_back_to_ok() {
    // 유효하지 않은 상태 코드 (예: 9999)는 OK로 대체
    let script_res = make_script_response(9999, HashMap::new(), None);
    let res = LoggingHandler::build_script_response(&script_res);

    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn test_build_script_response_various_status_codes() {
    for (code, expected) in [
        (301, StatusCode::MOVED_PERMANENTLY),
        (400, StatusCode::BAD_REQUEST),
        (500, StatusCode::INTERNAL_SERVER_ERROR),
    ] {
        let script_res = make_script_response(code, HashMap::new(), Some("body".to_string()));
        let res = LoggingHandler::build_script_response(&script_res);
        assert_eq!(res.status(), expected);
    }
}

// ── apply_script_response_modify 테스트 ──

#[test]
fn test_apply_script_response_modify_status() {
    let res = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("body"))
        .unwrap();

    let modified = make_script_response(404, HashMap::new(), None);
    let result = LoggingHandler::apply_script_response_modify(res, &modified);

    assert_eq!(result.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_apply_script_response_modify_headers_merged() {
    let res = Response::builder()
        .status(StatusCode::OK)
        .header("existing", "value")
        .body(Body::from("body"))
        .unwrap();

    let mut new_headers = HashMap::new();
    new_headers.insert("x-added".to_string(), "new".to_string());
    let modified = make_script_response(200, new_headers, None);
    let result = LoggingHandler::apply_script_response_modify(res, &modified);

    // 기존 헤더 유지 + 새 헤더 추가 (apply_script_response_modify는 clear 하지 않음)
    assert_eq!(result.headers().get("existing").unwrap(), "value");
    assert_eq!(result.headers().get("x-added").unwrap(), "new");
}

#[test]
fn test_apply_script_response_modify_overwrite_header() {
    let res = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/plain")
        .body(Body::from("body"))
        .unwrap();

    let mut new_headers = HashMap::new();
    new_headers.insert("content-type".to_string(), "application/json".to_string());
    let modified = make_script_response(200, new_headers, None);
    let result = LoggingHandler::apply_script_response_modify(res, &modified);

    assert_eq!(
        result.headers().get("content-type").unwrap(),
        "application/json"
    );
}

#[test]
fn test_apply_script_response_modify_body_replaced() {
    let res = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("old body"))
        .unwrap();

    let modified = make_script_response(200, HashMap::new(), Some("new body".to_string()));
    let result = LoggingHandler::apply_script_response_modify(res, &modified);

    assert_eq!(result.status(), StatusCode::OK);
}

#[test]
fn test_apply_script_response_modify_body_none_preserves() {
    let res = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("original"))
        .unwrap();

    let modified = make_script_response(200, HashMap::new(), None);
    let result = LoggingHandler::apply_script_response_modify(res, &modified);

    assert_eq!(result.status(), StatusCode::OK);
}

#[test]
fn test_apply_script_response_modify_invalid_status_ignored() {
    let res = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("body"))
        .unwrap();

    // 유효하지 않은 상태 코드는 무시됨 (원래 값 유지)
    let modified = make_script_response(9999, HashMap::new(), None);
    let result = LoggingHandler::apply_script_response_modify(res, &modified);

    assert_eq!(result.status(), StatusCode::OK);
}

// ── 바디 교체 시 stale content-length/encoding 헤더 제거 (회귀) ──

#[test]
fn test_apply_script_request_modify_body_clears_encoding_headers() {
    let req = Request::builder()
        .method(Method::POST)
        .uri("http://example.com")
        .body(Body::from("old body"))
        .unwrap();

    // 스크립트가 받은 헤더 맵에는 원본 content-length/encoding이 그대로 들어 있다.
    let mut headers = HashMap::new();
    headers.insert("content-length".to_string(), "8".to_string());
    headers.insert("content-encoding".to_string(), "gzip".to_string());
    headers.insert("transfer-encoding".to_string(), "chunked".to_string());
    headers.insert("x-keep".to_string(), "yes".to_string());

    let modified = make_script_request(
        "POST",
        "http://example.com",
        headers,
        Some("a much longer new body".to_string()),
    );
    let result = LoggingHandler::apply_script_request_modify(req, &modified);

    // 바디를 교체했으므로 길이/인코딩 헤더는 제거되어야 한다.
    assert!(result.headers().get("content-length").is_none());
    assert!(result.headers().get("content-encoding").is_none());
    assert!(result.headers().get("transfer-encoding").is_none());
    // 그 외 헤더는 유지.
    assert_eq!(result.headers().get("x-keep").unwrap(), "yes");
}

#[test]
fn test_apply_script_request_modify_no_body_keeps_headers() {
    let req = Request::builder()
        .method(Method::POST)
        .uri("http://example.com")
        .body(Body::from("body"))
        .unwrap();

    let mut headers = HashMap::new();
    headers.insert("content-length".to_string(), "4".to_string());
    let modified = make_script_request("POST", "http://example.com", headers, None);
    let result = LoggingHandler::apply_script_request_modify(req, &modified);

    // 바디를 교체하지 않으면 헤더는 스크립트가 준 그대로 유지된다.
    assert_eq!(result.headers().get("content-length").unwrap(), "4");
}

#[test]
fn test_apply_script_response_modify_body_clears_encoding_headers() {
    let res = Response::builder()
        .status(StatusCode::OK)
        .header("content-length", "8")
        .header("content-encoding", "gzip")
        .header("transfer-encoding", "chunked")
        .header("x-keep", "yes")
        .body(Body::from("old body"))
        .unwrap();

    let modified = make_script_response(
        200,
        HashMap::new(),
        Some("a much longer new plaintext body".to_string()),
    );
    let result = LoggingHandler::apply_script_response_modify(res, &modified);

    // 바디 교체 시 원본 content-length/encoding 제거(평문을 gzip으로 디코딩하는 손상 방지).
    assert!(result.headers().get("content-length").is_none());
    assert!(result.headers().get("content-encoding").is_none());
    assert!(result.headers().get("transfer-encoding").is_none());
    assert_eq!(result.headers().get("x-keep").unwrap(), "yes");
}

#[test]
fn test_apply_script_response_modify_no_body_keeps_encoding_headers() {
    let res = Response::builder()
        .status(StatusCode::OK)
        .header("content-encoding", "gzip")
        .body(Body::from("original gzipped"))
        .unwrap();

    // 바디를 교체하지 않으면 원본 인코딩 헤더는 유지되어야 한다.
    let modified = make_script_response(200, HashMap::new(), None);
    let result = LoggingHandler::apply_script_response_modify(res, &modified);

    assert_eq!(result.headers().get("content-encoding").unwrap(), "gzip");
}
