#[cfg(test)]
mod tests {
    use crate::protocol::*;
    use std::collections::HashMap;

    #[test]
    fn test_breakpoint_rule_serde_roundtrip() {
        let rule = BreakpointRule {
            id: "rule_1".to_string(),
            pattern: "*api.example.com*".to_string(),
            break_on_request: true,
            break_on_response: false,
            enabled: true,
        };
        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: BreakpointRule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, rule.id);
        assert_eq!(deserialized.pattern, rule.pattern);
        assert_eq!(deserialized.break_on_request, rule.break_on_request);
        assert_eq!(deserialized.break_on_response, rule.break_on_response);
        assert_eq!(deserialized.enabled, rule.enabled);
    }

    #[test]
    fn test_breakpoint_data_serde_roundtrip() {
        let data = BreakpointData {
            method: "POST".to_string(),
            url: "https://api.example.com/users".to_string(),
            headers: HashMap::from([
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Authorization".to_string(), "Bearer token".to_string()),
            ]),
            body: Some("{\"name\": \"test\"}".to_string()),
            status: Some(200),
        };
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: BreakpointData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.method, data.method);
        assert_eq!(deserialized.url, data.url);
        assert_eq!(deserialized.headers.len(), 2);
        assert_eq!(deserialized.body, data.body);
        assert_eq!(deserialized.status, data.status);
    }

    #[test]
    fn test_breakpoint_action_forward_serde() {
        let action = BreakpointAction::Forward;
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"forward\""));
        let deserialized: BreakpointAction = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, BreakpointAction::Forward));
    }

    #[test]
    fn test_breakpoint_action_modify_and_forward_serde() {
        let action = BreakpointAction::ModifyAndForward {
            headers: Some(HashMap::from([(
                "X-Custom".to_string(),
                "value".to_string(),
            )])),
            body: Some("modified body".to_string()),
            status: Some(201),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"modify_and_forward\""));
        let deserialized: BreakpointAction = serde_json::from_str(&json).unwrap();
        match deserialized {
            BreakpointAction::ModifyAndForward {
                headers,
                body,
                status,
            } => {
                assert_eq!(headers.unwrap().get("X-Custom").unwrap(), "value");
                assert_eq!(body.unwrap(), "modified body");
                assert_eq!(status.unwrap(), 201);
            }
            _ => panic!("Expected ModifyAndForward variant"),
        }
    }

    #[test]
    fn test_breakpoint_action_drop_serde() {
        let action = BreakpointAction::Drop;
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"drop\""));
        let deserialized: BreakpointAction = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, BreakpointAction::Drop));
    }

    #[test]
    fn test_breakpoint_action_abort_serde() {
        let action = BreakpointAction::Abort;
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"abort\""));
        let deserialized: BreakpointAction = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, BreakpointAction::Abort));
    }
}
