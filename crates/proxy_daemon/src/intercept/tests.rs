use super::helpers::*;
use crate::handler::LoggingHandler;
use crate::protocol::{InterceptAction, InterceptRule, RewriteTarget};
use proxyapi_v2::hyper::http::{HeaderMap, HeaderValue};
use proxyapi_v2::Body;
use regex::Regex;

// --- rule_matches 테스트 ---

fn make_rule(pattern: &str, method: Option<&str>, enabled: bool) -> InterceptRule {
    InterceptRule {
        id: "test".to_string(),
        name: "Test".to_string(),
        enabled,
        pattern: pattern.to_string(),
        method: method.map(|m| m.to_string()),
        action: InterceptAction::Block {
            status_code: 403,
            body: String::new(),
        },
    }
}

#[test]
fn test_rule_matches_basic() {
    let rule = make_rule("*example.com*", None, true);
    assert!(LoggingHandler::rule_matches(
        &rule,
        "https://example.com/api",
        "GET"
    ));
}

#[test]
fn test_rule_matches_with_method() {
    let rule = make_rule("*api.com*", Some("POST"), true);
    assert!(LoggingHandler::rule_matches(
        &rule,
        "https://api.com/v1",
        "POST"
    ));
    assert!(!LoggingHandler::rule_matches(
        &rule,
        "https://api.com/v1",
        "GET"
    ));
}

#[test]
fn test_rule_matches_method_case_insensitive() {
    let rule = make_rule("*api.com*", Some("post"), true);
    assert!(LoggingHandler::rule_matches(
        &rule,
        "https://api.com/v1",
        "POST"
    ));
}

#[test]
fn test_rule_matches_disabled() {
    let rule = make_rule("*example.com*", None, false);
    assert!(!LoggingHandler::rule_matches(
        &rule,
        "https://example.com/api",
        "GET"
    ));
}

#[test]
fn test_rule_matches_no_method_filter_matches_all() {
    let rule = make_rule("*example.com*", None, true);
    assert!(LoggingHandler::rule_matches(
        &rule,
        "https://example.com",
        "GET"
    ));
    assert!(LoggingHandler::rule_matches(
        &rule,
        "https://example.com",
        "POST"
    ));
    assert!(LoggingHandler::rule_matches(
        &rule,
        "https://example.com",
        "DELETE"
    ));
}

#[test]
fn test_rule_matches_complex_pattern() {
    let rule = make_rule("*.example.com/api/*/users", Some("GET"), true);
    assert!(LoggingHandler::rule_matches(
        &rule,
        "https://sub.example.com/api/v1/users",
        "GET"
    ));
    assert!(!LoggingHandler::rule_matches(
        &rule,
        "https://sub.example.com/api/v1/posts",
        "GET"
    ));
    assert!(!LoggingHandler::rule_matches(
        &rule,
        "https://sub.example.com/api/v1/users",
        "POST"
    ));
}

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

// --- Rewrite InterceptAction serde 테스트 ---

#[test]
fn test_rewrite_action_serde_roundtrip() {
    let action = InterceptAction::Rewrite {
        target: RewriteTarget::ResponseBody,
        match_pattern: r#""premium": false"#.to_string(),
        replace_with: r#""premium": true"#.to_string(),
    };
    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("\"type\":\"rewrite\""));
    assert!(json.contains("response_body"));

    let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
    match deserialized {
        InterceptAction::Rewrite {
            target,
            match_pattern,
            replace_with,
        } => {
            assert_eq!(target, RewriteTarget::ResponseBody);
            assert_eq!(match_pattern, r#""premium": false"#);
            assert_eq!(replace_with, r#""premium": true"#);
        }
        _ => panic!("Expected Rewrite variant"),
    }
}

#[test]
fn test_rewrite_action_all_targets() {
    let targets = vec![
        (RewriteTarget::RequestHeader, "request_header"),
        (RewriteTarget::ResponseHeader, "response_header"),
        (RewriteTarget::RequestBody, "request_body"),
        (RewriteTarget::ResponseBody, "response_body"),
    ];
    for (target, expected_str) in targets {
        let action = InterceptAction::Rewrite {
            target: target.clone(),
            match_pattern: "test".to_string(),
            replace_with: "replaced".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(
            json.contains(expected_str),
            "JSON should contain '{}' for target {:?}, got: {}",
            expected_str,
            target,
            json
        );
    }
}

#[test]
fn test_rewrite_rule_display() {
    let rule = InterceptRule {
        id: "rw_1".to_string(),
        name: "Rewrite Test".to_string(),
        enabled: true,
        pattern: "*example.com*".to_string(),
        method: None,
        action: InterceptAction::Rewrite {
            target: RewriteTarget::ResponseBody,
            match_pattern: "old".to_string(),
            replace_with: "new".to_string(),
        },
    };
    let display = format!("{}", rule);
    assert!(display.contains("Rewrite"));
    assert!(display.contains("old"));
}

// --- apply_header_modifications ---

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

// --- guess_content_type ---

#[test]
fn test_guess_content_type_json() {
    assert_eq!(
        LoggingHandler::guess_content_type("data.json"),
        "application/json"
    );
}

#[test]
fn test_guess_content_type_html() {
    assert_eq!(
        LoggingHandler::guess_content_type("index.html"),
        "text/html; charset=utf-8"
    );
    assert_eq!(
        LoggingHandler::guess_content_type("page.htm"),
        "text/html; charset=utf-8"
    );
}

#[test]
fn test_guess_content_type_javascript() {
    assert_eq!(
        LoggingHandler::guess_content_type("app.js"),
        "application/javascript"
    );
    assert_eq!(
        LoggingHandler::guess_content_type("module.mjs"),
        "application/javascript"
    );
}

#[test]
fn test_guess_content_type_css() {
    assert_eq!(LoggingHandler::guess_content_type("style.css"), "text/css");
}

#[test]
fn test_guess_content_type_images() {
    assert_eq!(LoggingHandler::guess_content_type("photo.png"), "image/png");
    assert_eq!(
        LoggingHandler::guess_content_type("photo.jpg"),
        "image/jpeg"
    );
    assert_eq!(
        LoggingHandler::guess_content_type("photo.jpeg"),
        "image/jpeg"
    );
    assert_eq!(LoggingHandler::guess_content_type("anim.gif"), "image/gif");
    assert_eq!(
        LoggingHandler::guess_content_type("icon.svg"),
        "image/svg+xml"
    );
    assert_eq!(
        LoggingHandler::guess_content_type("photo.webp"),
        "image/webp"
    );
}

#[test]
fn test_guess_content_type_fonts() {
    assert_eq!(
        LoggingHandler::guess_content_type("font.woff"),
        "font/woff2"
    );
    assert_eq!(
        LoggingHandler::guess_content_type("font.woff2"),
        "font/woff2"
    );
}

#[test]
fn test_guess_content_type_other() {
    assert_eq!(
        LoggingHandler::guess_content_type("file.bin"),
        "application/octet-stream"
    );
    assert_eq!(
        LoggingHandler::guess_content_type("noext"),
        "application/octet-stream"
    );
}

#[test]
fn test_guess_content_type_xml() {
    assert_eq!(
        LoggingHandler::guess_content_type("data.xml"),
        "application/xml"
    );
}

#[test]
fn test_guess_content_type_txt() {
    assert_eq!(
        LoggingHandler::guess_content_type("readme.txt"),
        "text/plain; charset=utf-8"
    );
}

#[test]
fn test_guess_content_type_pdf() {
    assert_eq!(
        LoggingHandler::guess_content_type("doc.pdf"),
        "application/pdf"
    );
}

#[test]
fn test_guess_content_type_case_insensitive() {
    assert_eq!(
        LoggingHandler::guess_content_type("DATA.JSON"),
        "application/json"
    );
    assert_eq!(LoggingHandler::guess_content_type("STYLE.CSS"), "text/css");
}

#[test]
fn test_guess_content_type_with_path() {
    assert_eq!(
        LoggingHandler::guess_content_type("/var/data/response.json"),
        "application/json"
    );
}

// --- rewrite_headers edge cases ---

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

// --- rewrite_body_bytes edge cases ---

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

// --- Block action serde ---

#[test]
fn test_block_action_serde_roundtrip() {
    let action = InterceptAction::Block {
        status_code: 403,
        body: "Forbidden".to_string(),
    };
    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("\"type\":\"block\""));

    let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
    match deserialized {
        InterceptAction::Block { status_code, body } => {
            assert_eq!(status_code, 403);
            assert_eq!(body, "Forbidden");
        }
        _ => panic!("Expected Block variant"),
    }
}

// --- ModifyRequest action serde ---

#[test]
fn test_modify_request_action_serde_roundtrip() {
    let mut add_headers = std::collections::HashMap::new();
    add_headers.insert("x-custom".to_string(), "value".to_string());
    let action = InterceptAction::ModifyRequest {
        add_headers,
        remove_headers: vec!["x-remove".to_string()],
        set_body: Some("new body".to_string()),
    };
    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("\"type\":\"modify_request\""));

    let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
    match deserialized {
        InterceptAction::ModifyRequest {
            add_headers,
            remove_headers,
            set_body,
        } => {
            assert_eq!(add_headers.get("x-custom").unwrap(), "value");
            assert_eq!(remove_headers, vec!["x-remove"]);
            assert_eq!(set_body.unwrap(), "new body");
        }
        _ => panic!("Expected ModifyRequest variant"),
    }
}

// --- ModifyResponse action serde ---

#[test]
fn test_modify_response_action_serde_roundtrip() {
    let action = InterceptAction::ModifyResponse {
        set_status: Some(201),
        add_headers: std::collections::HashMap::new(),
        remove_headers: vec![],
        set_body: None,
    };
    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("\"type\":\"modify_response\""));

    let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
    match deserialized {
        InterceptAction::ModifyResponse {
            set_status,
            set_body,
            ..
        } => {
            assert_eq!(set_status.unwrap(), 201);
            assert!(set_body.is_none());
        }
        _ => panic!("Expected ModifyResponse variant"),
    }
}

// --- MapLocal action serde ---

#[test]
fn test_map_local_action_serde_roundtrip() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("x-custom".to_string(), "val".to_string());
    let action = InterceptAction::MapLocal {
        file_path: "/tmp/mock.json".to_string(),
        status_code: 200,
        headers,
    };
    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("\"type\":\"map_local\""));

    let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
    match deserialized {
        InterceptAction::MapLocal {
            file_path,
            status_code,
            headers,
        } => {
            assert_eq!(file_path, "/tmp/mock.json");
            assert_eq!(status_code, 200);
            assert_eq!(headers.get("x-custom").unwrap(), "val");
        }
        _ => panic!("Expected MapLocal variant"),
    }
}

// --- MapRemote action serde ---

#[test]
fn test_map_remote_action_serde_roundtrip() {
    let action = InterceptAction::MapRemote {
        target_url: "http://localhost:3000".to_string(),
        preserve_path: true,
    };
    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("\"type\":\"map_remote\""));
    assert!(json.contains("\"preserve_path\":true"));

    let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
    match deserialized {
        InterceptAction::MapRemote {
            target_url,
            preserve_path,
        } => {
            assert_eq!(target_url, "http://localhost:3000");
            assert!(preserve_path);
        }
        _ => panic!("Expected MapRemote variant"),
    }
}

#[test]
fn test_map_remote_action_preserve_path_false() {
    let action = InterceptAction::MapRemote {
        target_url: "http://mock.local/fixed".to_string(),
        preserve_path: false,
    };
    let json = serde_json::to_string(&action).unwrap();
    let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
    match deserialized {
        InterceptAction::MapRemote { preserve_path, .. } => {
            assert!(!preserve_path);
        }
        _ => panic!("Expected MapRemote variant"),
    }
}

// --- ServerReplayEntry serde ---

#[test]
fn test_server_replay_entry_serde_roundtrip() {
    use crate::protocol::ServerReplayEntry;

    let mut headers = std::collections::HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());
    let entry = ServerReplayEntry {
        id: "sr_1".to_string(),
        method: "GET".to_string(),
        url: "https://api.example.com/users".to_string(),
        status: 200,
        headers,
        body: Some(r#"[{"id": 1}]"#.to_string()),
    };
    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: ServerReplayEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "sr_1");
    assert_eq!(deserialized.method, "GET");
    assert_eq!(deserialized.url, "https://api.example.com/users");
    assert_eq!(deserialized.status, 200);
    assert_eq!(
        deserialized.headers.get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(deserialized.body.unwrap(), r#"[{"id": 1}]"#);
}

#[test]
fn test_server_replay_entry_no_body() {
    use crate::protocol::ServerReplayEntry;

    let entry = ServerReplayEntry {
        id: "sr_2".to_string(),
        method: "DELETE".to_string(),
        url: "https://api.example.com/users/1".to_string(),
        status: 204,
        headers: std::collections::HashMap::new(),
        body: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: ServerReplayEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.status, 204);
    assert!(deserialized.body.is_none());
}

// --- InterceptRule Display for various actions ---

#[test]
fn test_block_rule_display() {
    let rule = InterceptRule {
        id: "b_1".to_string(),
        name: "Block Ads".to_string(),
        enabled: true,
        pattern: "*ads*".to_string(),
        method: None,
        action: InterceptAction::Block {
            status_code: 403,
            body: String::new(),
        },
    };
    let display = format!("{}", rule);
    assert!(display.contains("Block"));
    assert!(display.contains("403"));
}

#[test]
fn test_modify_request_rule_display() {
    let rule = InterceptRule {
        id: "mr_1".to_string(),
        name: "Add Auth".to_string(),
        enabled: true,
        pattern: "*api*".to_string(),
        method: Some("GET".to_string()),
        action: InterceptAction::ModifyRequest {
            add_headers: std::collections::HashMap::new(),
            remove_headers: vec![],
            set_body: None,
        },
    };
    let display = format!("{}", rule);
    assert!(display.contains("ModifyRequest"));
}

#[test]
fn test_modify_response_rule_display() {
    let rule = InterceptRule {
        id: "mres_1".to_string(),
        name: "Change Status".to_string(),
        enabled: false,
        pattern: "*".to_string(),
        method: None,
        action: InterceptAction::ModifyResponse {
            set_status: Some(500),
            add_headers: std::collections::HashMap::new(),
            remove_headers: vec![],
            set_body: None,
        },
    };
    let display = format!("{}", rule);
    assert!(display.contains("ModifyResponse"));
    assert!(display.contains("500"));
}

#[test]
fn test_map_local_rule_display() {
    let rule = InterceptRule {
        id: "ml_1".to_string(),
        name: "Local Mock".to_string(),
        enabled: true,
        pattern: "*api/users*".to_string(),
        method: None,
        action: InterceptAction::MapLocal {
            file_path: "/tmp/users.json".to_string(),
            status_code: 200,
            headers: std::collections::HashMap::new(),
        },
    };
    let display = format!("{}", rule);
    assert!(display.contains("MapLocal"));
    assert!(display.contains("/tmp/users.json"));
}

#[test]
fn test_map_remote_rule_display() {
    let rule = InterceptRule {
        id: "mr_1".to_string(),
        name: "Remote Redirect".to_string(),
        enabled: true,
        pattern: "*prod.api.com*".to_string(),
        method: None,
        action: InterceptAction::MapRemote {
            target_url: "http://localhost:8080".to_string(),
            preserve_path: true,
        },
    };
    let display = format!("{}", rule);
    assert!(display.contains("MapRemote"));
    assert!(display.contains("localhost:8080"));
}

// --- rule_matches edge cases ---

#[test]
fn test_rule_matches_pattern_no_match() {
    let rule = make_rule("*totally-different.com*", None, true);
    assert!(!LoggingHandler::rule_matches(
        &rule,
        "https://example.com/api",
        "GET"
    ));
}

#[test]
fn test_rule_matches_wildcard_question_mark() {
    let rule = make_rule("*api/v?/*", None, true);
    assert!(LoggingHandler::rule_matches(
        &rule,
        "https://example.com/api/v1/users",
        "GET"
    ));
    assert!(LoggingHandler::rule_matches(
        &rule,
        "https://example.com/api/v2/users",
        "GET"
    ));
    assert!(!LoggingHandler::rule_matches(
        &rule,
        "https://example.com/api/v10/users",
        "GET"
    ));
}

#[test]
fn test_rule_matches_catch_all_pattern() {
    let rule = make_rule("*", None, true);
    assert!(LoggingHandler::rule_matches(
        &rule,
        "https://anything.com/any/path",
        "GET"
    ));
}

#[test]
fn test_rule_matches_empty_url() {
    let rule = make_rule("*", None, true);
    assert!(LoggingHandler::rule_matches(&rule, "", "GET"));
}

#[test]
fn test_rule_matches_method_put_and_patch() {
    let put_rule = make_rule("*api*", Some("PUT"), true);
    assert!(LoggingHandler::rule_matches(
        &put_rule,
        "https://api.com/resource",
        "PUT"
    ));
    assert!(!LoggingHandler::rule_matches(
        &put_rule,
        "https://api.com/resource",
        "PATCH"
    ));

    let patch_rule = make_rule("*api*", Some("PATCH"), true);
    assert!(LoggingHandler::rule_matches(
        &patch_rule,
        "https://api.com/resource",
        "PATCH"
    ));
}

// --- Block action defaults serde ---

#[test]
fn test_block_action_default_status_code() {
    let json = r#"{"type":"block"}"#;
    let action: InterceptAction = serde_json::from_str(json).unwrap();
    match action {
        InterceptAction::Block { status_code, body } => {
            assert_eq!(status_code, 403);
            assert!(body.is_empty());
        }
        _ => panic!("Expected Block variant"),
    }
}

// --- ModifyRequest defaults serde ---

#[test]
fn test_modify_request_defaults() {
    let json = r#"{"type":"modify_request"}"#;
    let action: InterceptAction = serde_json::from_str(json).unwrap();
    match action {
        InterceptAction::ModifyRequest {
            add_headers,
            remove_headers,
            set_body,
        } => {
            assert!(add_headers.is_empty());
            assert!(remove_headers.is_empty());
            assert!(set_body.is_none());
        }
        _ => panic!("Expected ModifyRequest variant"),
    }
}

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

// --- ModifyResponse defaults serde ---

#[test]
fn test_modify_response_defaults() {
    let json = r#"{"type":"modify_response"}"#;
    let action: InterceptAction = serde_json::from_str(json).unwrap();
    match action {
        InterceptAction::ModifyResponse {
            set_status,
            add_headers,
            remove_headers,
            set_body,
        } => {
            assert!(set_status.is_none());
            assert!(add_headers.is_empty());
            assert!(remove_headers.is_empty());
            assert!(set_body.is_none());
        }
        _ => panic!("Expected ModifyResponse variant"),
    }
}
