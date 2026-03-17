use super::helpers;
use super::*;
use base64::Engine;
use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode, Uri, Version};

use crate::{ProxiedRequest, ProxiedResponse, RequestInfo};

fn make_request(method: &str, uri: &str, body: &[u8], time: i64) -> crate::ClientRequest {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert("host", "example.com".parse().unwrap());

    let req = ProxiedRequest::new(
        method.parse::<Method>().unwrap(),
        uri.parse::<Uri>().unwrap(),
        Version::HTTP_11,
        headers,
        Bytes::from(body.to_vec()),
        time,
    );
    req.for_client(None)
}

fn make_response(status: u16, body: &[u8], time: i64, req_id: &str) -> crate::ClientResponse {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());

    let res = ProxiedResponse::new(
        StatusCode::from_u16(status).unwrap(),
        Version::HTTP_11,
        headers,
        Bytes::from(body.to_vec()),
        time,
    );
    res.for_client(req_id, None)
}

fn make_transaction(
    method: &str,
    uri: &str,
    req_body: &[u8],
    status: u16,
    res_body: &[u8],
    req_time: i64,
    res_time: i64,
) -> RequestInfo {
    let req = make_request(method, uri, req_body, req_time);
    let id = req.id().to_string();
    let res = make_response(status, res_body, res_time, &id);
    RequestInfo {
        request: Some(req),
        response: Some(res),
        validations: None,
        server_cert: None,
        tls_fallback_used: None,
    }
}

#[test]
fn build_har_empty_transactions() {
    let har = build_har(&[]);
    assert_eq!(har.log.version, "1.2");
    assert_eq!(har.log.creator.name, "Cheolsu Proxy");
    assert!(har.log.entries.is_empty());
}

#[test]
fn build_har_single_transaction() {
    let tx = make_transaction(
        "GET",
        "https://example.com/api?q=test&page=1",
        b"",
        200,
        b"{\"ok\":true}",
        1700000000000,
        1700000000100,
    );

    let har = build_har(&[tx]);
    assert_eq!(har.log.entries.len(), 1);

    let entry = &har.log.entries[0];
    assert_eq!(entry.request.method, "GET");
    assert_eq!(entry.request.url, "https://example.com/api?q=test&page=1");
    assert_eq!(entry.request.http_version, "HTTP/1.1");
    assert_eq!(entry.response.status, 200);
    assert_eq!(entry.response.status_text, "OK");
    assert_eq!(entry.time, 100);
    assert_eq!(entry.timings.wait, 100);
}

#[test]
fn build_har_query_string_parsed() {
    let tx = make_transaction(
        "GET",
        "https://example.com/search?q=hello&lang=ko",
        b"",
        200,
        b"",
        1700000000000,
        1700000000050,
    );

    let har = build_har(&[tx]);
    let qs = &har.log.entries[0].request.query_string;
    assert_eq!(qs.len(), 2);
    assert_eq!(qs[0].name, "q");
    assert_eq!(qs[0].value, "hello");
    assert_eq!(qs[1].name, "lang");
    assert_eq!(qs[1].value, "ko");
}

#[test]
fn build_har_post_data_included() {
    let body = b"{\"name\":\"test\"}";
    let tx = make_transaction(
        "POST",
        "https://example.com/api",
        body,
        201,
        b"",
        1700000000000,
        1700000000050,
    );

    let har = build_har(&[tx]);
    let post_data = har.log.entries[0].request.post_data.as_ref().unwrap();
    assert!(post_data.mime_type.contains("json"));
    assert_eq!(post_data.text, "{\"name\":\"test\"}");
}

#[test]
fn build_har_binary_body_base64_encoded() {
    let binary_body: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0xFF, 0xFE, 0x00, 0x01];
    let tx = make_transaction(
        "GET",
        "https://example.com/image",
        b"",
        200,
        &binary_body,
        1700000000000,
        1700000000050,
    );

    let har = build_har(&[tx]);
    let content = &har.log.entries[0].response.content;
    assert_eq!(content.encoding.as_deref(), Some("base64"));
    assert!(content.text.is_some());
}

#[test]
fn build_har_text_body_no_encoding() {
    let tx = make_transaction(
        "GET",
        "https://example.com/api",
        b"",
        200,
        b"{\"result\":42}",
        1700000000000,
        1700000000050,
    );

    let har = build_har(&[tx]);
    let content = &har.log.entries[0].response.content;
    assert!(content.encoding.is_none());
    assert_eq!(content.text.as_deref(), Some("{\"result\":42}"));
}

#[test]
fn build_har_request_only_no_response() {
    let req = make_request("GET", "https://example.com/pending", b"", 1700000000000);
    let tx = RequestInfo {
        request: Some(req),
        response: None,
        validations: None,
        server_cert: None,
        tls_fallback_used: None,
    };

    let har = build_har(&[tx]);
    assert_eq!(har.log.entries.len(), 1);
    let entry = &har.log.entries[0];
    assert_eq!(entry.response.status, 0);
    assert_eq!(entry.response.body_size, -1);
    assert_eq!(entry.time, 0);
}

#[test]
fn build_har_no_request_skipped() {
    let tx = RequestInfo {
        request: None,
        response: None,
        validations: None,
        server_cert: None,
        tls_fallback_used: None,
    };
    let har = build_har(&[tx]);
    assert!(har.log.entries.is_empty());
}

#[test]
fn build_har_headers_serialized() {
    let tx = make_transaction(
        "GET",
        "https://example.com/",
        b"",
        200,
        b"ok",
        1700000000000,
        1700000000050,
    );

    let har = build_har(&[tx]);
    let req_headers = &har.log.entries[0].request.headers;
    assert!(req_headers.iter().any(|h| h.name == "content-type"));
    assert!(req_headers.iter().any(|h| h.name == "host"));
}

#[test]
fn build_har_cookies_parsed() {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "text/html".parse().unwrap());
    headers.insert("cookie", "sid=abc123; lang=ko".parse().unwrap());

    let req = ProxiedRequest::new(
        Method::GET,
        "https://example.com/".parse().unwrap(),
        Version::HTTP_11,
        headers,
        Bytes::new(),
        1700000000000,
    );
    let client_req = req.for_client(None);
    let tx = RequestInfo {
        request: Some(client_req),
        response: None,
        validations: None,
        server_cert: None,
        tls_fallback_used: None,
    };

    let har = build_har(&[tx]);
    let cookies = &har.log.entries[0].request.cookies;
    assert_eq!(cookies.len(), 2);
    assert_eq!(cookies[0].name, "sid");
    assert_eq!(cookies[0].value, "abc123");
    assert_eq!(cookies[1].name, "lang");
    assert_eq!(cookies[1].value, "ko");
}

#[test]
fn build_har_redirect_url() {
    let mut res_headers = HeaderMap::new();
    res_headers.insert("content-type", "text/html".parse().unwrap());
    res_headers.insert("location", "https://example.com/new".parse().unwrap());

    let req = make_request("GET", "https://example.com/old", b"", 1700000000000);
    let id = req.id().to_string();
    let res = ProxiedResponse::new(
        StatusCode::MOVED_PERMANENTLY,
        Version::HTTP_11,
        res_headers,
        Bytes::new(),
        1700000000050,
    );
    let client_res = res.for_client(&id, None);
    let tx = RequestInfo {
        request: Some(req),
        response: Some(client_res),
        validations: None,
        server_cert: None,
        tls_fallback_used: None,
    };

    let har = build_har(&[tx]);
    assert_eq!(
        har.log.entries[0].response.redirect_url,
        "https://example.com/new"
    );
    assert_eq!(har.log.entries[0].response.status, 301);
}

#[test]
fn build_har_multiple_transactions() {
    let txs = vec![
        make_transaction(
            "GET",
            "https://example.com/1",
            b"",
            200,
            b"ok",
            1700000000000,
            1700000000050,
        ),
        make_transaction(
            "POST",
            "https://example.com/2",
            b"data",
            201,
            b"created",
            1700000001000,
            1700000001200,
        ),
        make_transaction(
            "DELETE",
            "https://example.com/3",
            b"",
            404,
            b"",
            1700000002000,
            1700000002010,
        ),
    ];

    let har = build_har(&txs);
    assert_eq!(har.log.entries.len(), 3);
    assert_eq!(har.log.entries[0].request.method, "GET");
    assert_eq!(har.log.entries[1].request.method, "POST");
    assert_eq!(har.log.entries[2].request.method, "DELETE");
    assert_eq!(har.log.entries[2].response.status, 404);
}

#[test]
fn build_har_json_valid() {
    let tx = make_transaction(
        "GET",
        "https://example.com/",
        b"",
        200,
        b"ok",
        1700000000000,
        1700000000050,
    );

    let json = build_har_json(&[tx]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["log"]["version"], "1.2");
    assert_eq!(parsed["log"]["entries"].as_array().unwrap().len(), 1);
}

#[test]
fn build_har_started_date_time_iso_format() {
    let tx = make_transaction(
        "GET",
        "https://example.com/",
        b"",
        200,
        b"",
        1700000000000,
        1700000000000,
    );

    let har = build_har(&[tx]);
    let dt = &har.log.entries[0].started_date_time;
    // ISO 8601 형식 검증
    assert!(dt.contains("T"));
    assert!(dt.ends_with("Z") || dt.contains("+"));
}

// -- 내부 함수 테스트 --

#[test]
fn parse_query_string_empty() {
    assert!(helpers::parse_query_string("https://example.com/path").is_empty());
}

#[test]
fn parse_query_string_with_fragment() {
    let qs = helpers::parse_query_string("https://example.com/path?a=1&b=2#section");
    assert_eq!(qs.len(), 2);
    assert_eq!(qs[1].value, "2");
}

#[test]
fn parse_query_string_no_value() {
    let qs = helpers::parse_query_string("https://example.com/?flag");
    assert_eq!(qs.len(), 1);
    assert_eq!(qs[0].name, "flag");
    assert_eq!(qs[0].value, "");
}

#[test]
fn encode_body_text() {
    let (text, encoding) = helpers::encode_body(b"hello world");
    assert_eq!(text, "hello world");
    assert!(encoding.is_none());
}

#[test]
fn encode_body_binary() {
    let (text, encoding) = helpers::encode_body(&[0xFF, 0xFE, 0x00]);
    assert_eq!(encoding.as_deref(), Some("base64"));
    // base64 디코딩 검증
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&text)
        .unwrap();
    assert_eq!(decoded, vec![0xFF, 0xFE, 0x00]);
}

#[test]
fn status_text_known_codes() {
    assert_eq!(helpers::status_text(200), "OK");
    assert_eq!(helpers::status_text(404), "Not Found");
    assert_eq!(helpers::status_text(500), "Internal Server Error");
}

#[test]
fn status_text_unknown_code() {
    assert_eq!(helpers::status_text(418), "");
}

#[test]
fn http_version_string_variants() {
    assert_eq!(helpers::http_version_string(&Version::HTTP_10), "HTTP/1.0");
    assert_eq!(helpers::http_version_string(&Version::HTTP_11), "HTTP/1.1");
    assert_eq!(helpers::http_version_string(&Version::HTTP_2), "HTTP/2.0");
}

#[test]
fn compute_headers_size_empty() {
    let headers = HeaderMap::new();
    assert_eq!(helpers::compute_headers_size(&headers), -1);
}

#[test]
fn compute_headers_size_with_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("host", "example.com".parse().unwrap());
    let size = helpers::compute_headers_size(&headers);
    // "host: example.com\r\n" = 4 + 2 + 11 + 2 = 19, + final \r\n = 21
    assert_eq!(size, 21);
}

#[test]
fn parse_cookies_empty() {
    let headers = HeaderMap::new();
    assert!(helpers::parse_cookies(&headers, "cookie").is_empty());
}

#[test]
fn parse_cookies_multiple() {
    let mut headers = HeaderMap::new();
    headers.insert("cookie", "a=1; b=2; c=3".parse().unwrap());
    let cookies = helpers::parse_cookies(&headers, "cookie");
    assert_eq!(cookies.len(), 3);
    assert_eq!(cookies[0].name, "a");
    assert_eq!(cookies[2].value, "3");
}
