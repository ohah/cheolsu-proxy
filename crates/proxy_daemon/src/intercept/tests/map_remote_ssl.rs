use crate::intercept::actions::map_remote_needs_ssl_intercept;
use crate::protocol::{InterceptAction, InterceptRule};

fn make_map_remote_rule(
    pattern: &str,
    target_url: &str,
    enabled: bool,
) -> InterceptRule {
    InterceptRule {
        id: "test".to_string(),
        name: "Test MapRemote".to_string(),
        enabled,
        pattern: pattern.to_string(),
        method: None,
        action: InterceptAction::MapRemote {
            target_url: target_url.to_string(),
            preserve_path: true,
        },
    }
}

#[test]
fn test_map_remote_port_match() {
    // MapRemote: *pos-webview.payhere.dev* → http://localhost:8083
    // CONNECT: pos-webview.payhere.dev:8083 → should intercept
    let rules = vec![make_map_remote_rule(
        "*pos-webview.payhere.dev*",
        "http://localhost:8083",
        true,
    )];
    assert!(map_remote_needs_ssl_intercept(
        &rules,
        "pos-webview.payhere.dev",
        Some(8083),
    ));
}

#[test]
fn test_map_remote_port_mismatch() {
    // CONNECT port 443 doesn't match target port 8083
    let rules = vec![make_map_remote_rule(
        "*pos-webview.payhere.dev*",
        "http://localhost:8083",
        true,
    )];
    assert!(!map_remote_needs_ssl_intercept(
        &rules,
        "pos-webview.payhere.dev",
        Some(443),
    ));
}

#[test]
fn test_map_remote_host_mismatch() {
    // Host doesn't match pattern
    let rules = vec![make_map_remote_rule(
        "*pos-webview.payhere.dev*",
        "http://localhost:8083",
        true,
    )];
    assert!(!map_remote_needs_ssl_intercept(
        &rules,
        "other-domain.com",
        Some(8083),
    ));
}

#[test]
fn test_map_remote_disabled_rule_ignored() {
    let rules = vec![make_map_remote_rule(
        "*pos-webview.payhere.dev*",
        "http://localhost:8083",
        false,
    )];
    assert!(!map_remote_needs_ssl_intercept(
        &rules,
        "pos-webview.payhere.dev",
        Some(8083),
    ));
}

#[test]
fn test_map_remote_no_port_in_target() {
    // target_url에 포트가 명시되지 않은 경우 (기본 포트 사용)
    let rules = vec![make_map_remote_rule(
        "*example.com*",
        "http://localhost",
        true,
    )];
    assert!(!map_remote_needs_ssl_intercept(
        &rules,
        "example.com",
        Some(80),
    ));
}

#[test]
fn test_map_remote_no_connect_port() {
    // CONNECT에 포트가 없는 경우
    let rules = vec![make_map_remote_rule(
        "*example.com*",
        "http://localhost:8083",
        true,
    )];
    assert!(!map_remote_needs_ssl_intercept(
        &rules,
        "example.com",
        None,
    ));
}

#[test]
fn test_map_remote_multiple_rules() {
    let rules = vec![
        make_map_remote_rule("*api.example.com*", "http://localhost:3000", true),
        make_map_remote_rule("*web.example.com*", "http://localhost:8080", true),
    ];
    // First rule's port with second rule's host → no match
    assert!(!map_remote_needs_ssl_intercept(
        &rules,
        "web.example.com",
        Some(3000),
    ));
    // Correct combinations
    assert!(map_remote_needs_ssl_intercept(
        &rules,
        "api.example.com",
        Some(3000),
    ));
    assert!(map_remote_needs_ssl_intercept(
        &rules,
        "web.example.com",
        Some(8080),
    ));
}

#[test]
fn test_map_remote_non_map_remote_rule_ignored() {
    // Block rule with same pattern should not trigger intercept
    let rules = vec![InterceptRule {
        id: "test".to_string(),
        name: "Block rule".to_string(),
        enabled: true,
        pattern: "*example.com*".to_string(),
        method: None,
        action: InterceptAction::Block {
            status_code: 403,
            body: String::new(),
        },
    }];
    assert!(!map_remote_needs_ssl_intercept(
        &rules,
        "example.com",
        Some(8083),
    ));
}

#[test]
fn test_map_remote_subdomain_pattern() {
    // Wildcard subdomain pattern
    let rules = vec![make_map_remote_rule(
        "*.payhere.dev*",
        "http://localhost:8083",
        true,
    )];
    assert!(map_remote_needs_ssl_intercept(
        &rules,
        "pos-webview.payhere.dev",
        Some(8083),
    ));
}
