use crate::handler::LoggingHandler;
use proxyapi_v2::Body;

// --- apply_block 테스트 ---

#[test]
fn test_apply_block_returns_correct_status() {
    let response = LoggingHandler::apply_block(403, "", "GET", "https://example.com", "test-rule");
    assert_eq!(response.status().as_u16(), 403);
}

#[test]
fn test_apply_block_with_custom_status() {
    let response =
        LoggingHandler::apply_block(429, "Rate limited", "POST", "https://api.com", "rate-limit");
    assert_eq!(response.status().as_u16(), 429);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "text/plain; charset=utf-8"
    );
}

#[test]
fn test_apply_block_json_body_sets_json_content_type() {
    let response = LoggingHandler::apply_block(
        403,
        r#"{"error": "blocked"}"#,
        "GET",
        "https://example.com",
        "block-json",
    );
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/json"
    );
}

#[test]
fn test_apply_block_array_body_sets_json_content_type() {
    let response = LoggingHandler::apply_block(
        403,
        r#"[{"error": "blocked"}]"#,
        "GET",
        "https://example.com",
        "block-array",
    );
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/json"
    );
}

#[test]
fn test_apply_block_empty_body_no_content_type() {
    let response = LoggingHandler::apply_block(403, "", "GET", "https://example.com", "test-rule");
    assert!(response.headers().get("content-type").is_none());
}

#[test]
fn test_apply_block_invalid_status_falls_back_to_403() {
    let response = LoggingHandler::apply_block(9999, "", "GET", "https://example.com", "test-rule");
    assert_eq!(response.status().as_u16(), 403);
}

// --- apply_map_local 테스트 ---

#[test]
fn test_apply_map_local_missing_file_returns_404() {
    let headers = std::collections::HashMap::new();
    let response = LoggingHandler::apply_map_local(
        "/nonexistent/path/file.json",
        200,
        &headers,
        "GET",
        "https://example.com/api",
        "map-local-test",
    );
    assert_eq!(response.status().as_u16(), 404);
    assert!(response
        .headers()
        .get("x-cheolsu-map-local-error")
        .is_some());
}

#[test]
fn test_apply_map_local_with_existing_file() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("mock.json");
    let mut file = std::fs::File::create(&file_path).unwrap();
    write!(file, r#"{{"status": "ok"}}"#).unwrap();

    let headers = std::collections::HashMap::new();
    let response = LoggingHandler::apply_map_local(
        file_path.to_str().unwrap(),
        200,
        &headers,
        "GET",
        "https://example.com/api",
        "map-local-test",
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/json"
    );
    assert!(response.headers().get("x-cheolsu-map-local").is_some());
}

#[test]
fn test_apply_map_local_custom_content_type() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("data.bin");
    let mut file = std::fs::File::create(&file_path).unwrap();
    write!(file, "binary data").unwrap();

    let mut headers = std::collections::HashMap::new();
    headers.insert("content-type".to_string(), "text/xml".to_string());
    let response = LoggingHandler::apply_map_local(
        file_path.to_str().unwrap(),
        201,
        &headers,
        "POST",
        "https://example.com/upload",
        "map-local-custom",
    );
    assert_eq!(response.status().as_u16(), 201);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "text/xml"
    );
}

// --- apply_map_remote 테스트 ---

#[test]
fn test_apply_map_remote_changes_uri() {
    use proxyapi_v2::hyper::Request;

    let mut req = Request::builder()
        .uri("https://prod.api.com/v1/users")
        .body(Body::empty())
        .unwrap();

    LoggingHandler::apply_map_remote(
        &mut req,
        "http://localhost:3000",
        false,
        "https://prod.api.com/v1/users",
        "GET",
        "map-remote-test",
    );

    assert_eq!(req.uri().to_string(), "http://localhost:3000/");
}

#[test]
fn test_apply_map_remote_preserve_path() {
    use proxyapi_v2::hyper::Request;

    let mut req = Request::builder()
        .uri("https://prod.api.com/v1/users?page=1")
        .body(Body::empty())
        .unwrap();

    LoggingHandler::apply_map_remote(
        &mut req,
        "http://localhost:3000",
        true,
        "https://prod.api.com/v1/users?page=1",
        "GET",
        "map-remote-preserve",
    );

    assert_eq!(
        req.uri().to_string(),
        "http://localhost:3000/v1/users?page=1"
    );
}

#[test]
fn test_apply_map_remote_sets_host_header() {
    use proxyapi_v2::hyper::Request;

    let mut req = Request::builder()
        .uri("https://prod.api.com/v1/users")
        .body(Body::empty())
        .unwrap();

    LoggingHandler::apply_map_remote(
        &mut req,
        "http://localhost:3000/fixed",
        false,
        "https://prod.api.com/v1/users",
        "GET",
        "map-remote-host",
    );

    assert_eq!(
        req.headers()
            .get(proxyapi_v2::hyper::header::HOST)
            .unwrap()
            .to_str()
            .unwrap(),
        "localhost"
    );
}

#[test]
fn test_apply_map_remote_sets_original_header() {
    use proxyapi_v2::hyper::Request;

    let mut req = Request::builder()
        .uri("https://prod.api.com/v1/users")
        .body(Body::empty())
        .unwrap();

    LoggingHandler::apply_map_remote(
        &mut req,
        "http://localhost:3000",
        false,
        "https://prod.api.com/v1/users",
        "GET",
        "map-remote-original",
    );

    assert_eq!(
        req.headers()
            .get("x-cheolsu-map-remote-original")
            .unwrap()
            .to_str()
            .unwrap(),
        "https://prod.api.com/v1/users"
    );
}
