use crate::protocol::{InterceptAction, RewriteTarget};

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
