use super::super::helpers::*;
use proxyapi_v2::hyper::http::{HeaderMap, HeaderValue};

#[test]
fn test_apply_header_modifications_add_headers() {
    let mut headers = HeaderMap::new();
    let remove = vec![];
    let mut add = std::collections::HashMap::new();
    add.insert("x-added".to_string(), "value1".to_string());
    add.insert("x-another".to_string(), "value2".to_string());

    apply_header_modifications(&mut headers, &remove, &add);

    assert_eq!(headers.get("x-added").unwrap().to_str().unwrap(), "value1");
    assert_eq!(
        headers.get("x-another").unwrap().to_str().unwrap(),
        "value2"
    );
}

#[test]
fn test_apply_header_modifications_remove_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("x-remove-me", HeaderValue::from_static("bye"));
    headers.insert("x-keep", HeaderValue::from_static("stay"));

    let remove = vec!["x-remove-me".to_string()];
    let add = std::collections::HashMap::new();

    apply_header_modifications(&mut headers, &remove, &add);

    assert!(headers.get("x-remove-me").is_none());
    assert_eq!(headers.get("x-keep").unwrap().to_str().unwrap(), "stay");
}

#[test]
fn test_apply_header_modifications_add_and_remove() {
    let mut headers = HeaderMap::new();
    headers.insert("authorization", HeaderValue::from_static("Bearer old"));

    let remove = vec!["authorization".to_string()];
    let mut add = std::collections::HashMap::new();
    add.insert("authorization".to_string(), "Bearer new".to_string());

    apply_header_modifications(&mut headers, &remove, &add);

    assert_eq!(
        headers.get("authorization").unwrap().to_str().unwrap(),
        "Bearer new"
    );
}

#[test]
fn test_apply_header_modifications_invalid_header_name_ignored() {
    let mut headers = HeaderMap::new();
    let remove = vec!["invalid header name with spaces".to_string()];
    let add = std::collections::HashMap::new();

    apply_header_modifications(&mut headers, &remove, &add);
    assert!(headers.is_empty());
}

#[test]
fn test_apply_header_modifications_empty_operations() {
    let mut headers = HeaderMap::new();
    headers.insert("x-existing", HeaderValue::from_static("value"));

    apply_header_modifications(&mut headers, &vec![], &std::collections::HashMap::new());

    assert_eq!(
        headers.get("x-existing").unwrap().to_str().unwrap(),
        "value"
    );
}
