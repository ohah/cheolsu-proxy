use super::super::helpers::*;
use proxyapi_v2::hyper::http::{HeaderMap, HeaderValue};
use proxyapi_v2::Body;
use regex::Regex;

// --- rewrite_headers 테스트 ---

#[test]
fn test_rewrite_headers_simple_replacement() {
    let mut headers = HeaderMap::new();
    headers.insert("x-custom", HeaderValue::from_static("old-value"));
    headers.insert("x-other", HeaderValue::from_static("unchanged"));

    let re = Regex::new("old").unwrap();
    let replacements = rewrite_headers(&headers, &re, "new");

    assert_eq!(replacements.len(), 1);
    assert_eq!(replacements[0].0.as_str(), "x-custom");
    assert_eq!(replacements[0].1.to_str().unwrap(), "new-value");
}

#[test]
fn test_rewrite_headers_multiple_matches() {
    let mut headers = HeaderMap::new();
    headers.insert("x-first", HeaderValue::from_static("foo-bar"));
    headers.insert("x-second", HeaderValue::from_static("foo-baz"));
    headers.insert("x-third", HeaderValue::from_static("no-match"));

    let re = Regex::new("foo").unwrap();
    let replacements = rewrite_headers(&headers, &re, "qux");

    assert_eq!(replacements.len(), 2);
    let values: Vec<String> = replacements
        .iter()
        .map(|(_, v)| v.to_str().unwrap().to_string())
        .collect();
    assert!(values.contains(&"qux-bar".to_string()));
    assert!(values.contains(&"qux-baz".to_string()));
}

#[test]
fn test_rewrite_headers_regex_capture_groups() {
    let mut headers = HeaderMap::new();
    headers.insert("x-version", HeaderValue::from_static("v1.2.3"));

    let re = Regex::new(r"v(\d+)\.(\d+)\.(\d+)").unwrap();
    let replacements = rewrite_headers(&headers, &re, "version-$1-$2-$3");

    assert_eq!(replacements.len(), 1);
    assert_eq!(replacements[0].1.to_str().unwrap(), "version-1-2-3");
}

#[test]
fn test_rewrite_headers_no_match() {
    let mut headers = HeaderMap::new();
    headers.insert("x-custom", HeaderValue::from_static("hello"));

    let re = Regex::new("xyz").unwrap();
    let replacements = rewrite_headers(&headers, &re, "replaced");

    assert!(replacements.is_empty());
}

#[test]
fn test_rewrite_headers_empty_headermap() {
    let headers = HeaderMap::new();
    let re = Regex::new("anything").unwrap();
    let replacements = rewrite_headers(&headers, &re, "replaced");
    assert!(replacements.is_empty());
}

#[test]
fn test_rewrite_headers_replace_all_occurrences_in_value() {
    let mut headers = HeaderMap::new();
    headers.insert("x-data", HeaderValue::from_static("aaa-aaa-aaa"));

    let re = Regex::new("aaa").unwrap();
    let replacements = rewrite_headers(&headers, &re, "bbb");

    assert_eq!(replacements.len(), 1);
    assert_eq!(replacements[0].1.to_str().unwrap(), "bbb-bbb-bbb");
}

// --- rewrite_body_bytes 테스트 ---

#[tokio::test]
async fn test_rewrite_body_simple_replacement() {
    let body_content = r#"{"premium": false}"#;
    let mut body = Body::from(body_content);

    let re = Regex::new(r#""premium": false"#).unwrap();
    let result = rewrite_body_bytes(&mut body, &re, r#""premium": true"#).await;

    assert!(result.is_some());
    let new_body = String::from_utf8(result.unwrap().to_vec()).unwrap();
    assert_eq!(new_body, r#"{"premium": true}"#);
}

#[tokio::test]
async fn test_rewrite_body_regex_capture_groups() {
    let body_content = "123-456";
    let mut body = Body::from(body_content);

    let re = Regex::new(r"(\d+)-(\d+)").unwrap();
    let result = rewrite_body_bytes(&mut body, &re, "$2-$1").await;

    assert!(result.is_some());
    let new_body = String::from_utf8(result.unwrap().to_vec()).unwrap();
    assert_eq!(new_body, "456-123");
}

#[tokio::test]
async fn test_rewrite_body_multiple_occurrences() {
    let body_content = "aaa-bbb ccc-ddd";
    let mut body = Body::from(body_content);

    let re = Regex::new(r"(\w+)-(\w+)").unwrap();
    let result = rewrite_body_bytes(&mut body, &re, "$2-$1").await;

    assert!(result.is_some());
    let new_body = String::from_utf8(result.unwrap().to_vec()).unwrap();
    assert_eq!(new_body, "bbb-aaa ddd-ccc");
}

#[tokio::test]
async fn test_rewrite_body_no_match() {
    let body_content = "hello world";
    let mut body = Body::from(body_content);

    let re = Regex::new("xyz").unwrap();
    let result = rewrite_body_bytes(&mut body, &re, "replaced").await;

    // 매칭이 없어도 원본 바디가 반환됨 (replace_all은 매칭 없으면 원본 반환)
    assert!(result.is_some());
    let new_body = String::from_utf8(result.unwrap().to_vec()).unwrap();
    assert_eq!(new_body, "hello world");
}

#[tokio::test]
async fn test_rewrite_body_empty_body() {
    let mut body = Body::from("");

    let re = Regex::new("anything").unwrap();
    let result = rewrite_body_bytes(&mut body, &re, "replaced").await;

    assert!(result.is_some());
    let new_body = String::from_utf8(result.unwrap().to_vec()).unwrap();
    assert_eq!(new_body, "");
}

#[tokio::test]
async fn test_rewrite_body_large_replace_all() {
    let body_content = "apple banana apple cherry apple";
    let mut body = Body::from(body_content);

    let re = Regex::new("apple").unwrap();
    let result = rewrite_body_bytes(&mut body, &re, "orange").await;

    assert!(result.is_some());
    let new_body = String::from_utf8(result.unwrap().to_vec()).unwrap();
    assert_eq!(new_body, "orange banana orange cherry orange");
}

// --- 잘못된 정규식 에러 처리 테스트 ---

#[test]
fn test_invalid_regex_does_not_crash() {
    let result = Regex::new("[invalid");
    assert!(result.is_err(), "잘못된 정규식은 Err를 반환해야 합니다");
}

#[test]
fn test_invalid_regex_error_message() {
    let result = Regex::new("[invalid");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(!err_msg.is_empty(), "에러 메시지가 비어있지 않아야 합니다");
}
