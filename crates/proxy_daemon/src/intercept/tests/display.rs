use crate::protocol::{InterceptAction, InterceptRule, RewriteTarget};

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
