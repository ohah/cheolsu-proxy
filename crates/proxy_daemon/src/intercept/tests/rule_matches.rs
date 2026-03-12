use crate::handler::LoggingHandler;
use crate::protocol::{InterceptAction, InterceptRule};

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
