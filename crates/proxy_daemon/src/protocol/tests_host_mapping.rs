#[cfg(test)]
mod tests {
    use crate::protocol::*;

    #[test]
    fn test_host_mapping_serialize_deserialize_roundtrip() {
        let mapping = HostMapping {
            id: "hm_1".to_string(),
            source_host: "*.api.example.com".to_string(),
            source_port: Some(443),
            target_host: "192.168.1.100".to_string(),
            target_port: Some(8443),
            enabled: true,
        };
        let json = serde_json::to_string(&mapping).unwrap();
        let deserialized: HostMapping = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "hm_1");
        assert_eq!(deserialized.source_host, "*.api.example.com");
        assert_eq!(deserialized.source_port, Some(443));
        assert_eq!(deserialized.target_host, "192.168.1.100");
        assert_eq!(deserialized.target_port, Some(8443));
        assert!(deserialized.enabled);
    }

    #[test]
    fn test_host_mapping_roundtrip_no_ports() {
        let mapping = HostMapping {
            id: "hm_2".to_string(),
            source_host: "example.com".to_string(),
            source_port: None,
            target_host: "10.0.0.1".to_string(),
            target_port: None,
            enabled: false,
        };
        let json = serde_json::to_string(&mapping).unwrap();
        let deserialized: HostMapping = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "hm_2");
        assert!(deserialized.source_port.is_none());
        assert!(deserialized.target_port.is_none());
        assert!(!deserialized.enabled);
    }

    #[test]
    fn test_update_host_mappings_command_serialize() {
        let cmd = ClientCommand::UpdateHostMappings {
            mappings: vec![
                HostMapping {
                    id: "hm_1".to_string(),
                    source_host: "api.example.com".to_string(),
                    source_port: None,
                    target_host: "10.0.0.1".to_string(),
                    target_port: Some(8080),
                    enabled: true,
                },
                HostMapping {
                    id: "hm_2".to_string(),
                    source_host: "*.staging.com".to_string(),
                    source_port: Some(443),
                    target_host: "192.168.1.50".to_string(),
                    target_port: None,
                    enabled: false,
                },
            ],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_host_mappings"));
        assert!(json.contains("api.example.com"));
        assert!(json.contains("*.staging.com"));

        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateHostMappings { mappings } => {
                assert_eq!(mappings.len(), 2);
                assert_eq!(mappings[0].id, "hm_1");
                assert_eq!(mappings[1].source_host, "*.staging.com");
            }
            _ => panic!("Expected UpdateHostMappings"),
        }
    }

    #[test]
    fn test_host_mappings_updated_message_serialize() {
        let msg = DaemonMessage::HostMappingsUpdated {
            mappings: vec![HostMapping {
                id: "hm_1".to_string(),
                source_host: "example.com".to_string(),
                source_port: None,
                target_host: "127.0.0.1".to_string(),
                target_port: Some(3000),
                enabled: true,
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("host_mappings_updated"));
        assert!(json.contains("example.com"));

        let deserialized: DaemonMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            DaemonMessage::HostMappingsUpdated { mappings } => {
                assert_eq!(mappings.len(), 1);
                assert_eq!(mappings[0].target_host, "127.0.0.1");
                assert_eq!(mappings[0].target_port, Some(3000));
            }
            _ => panic!("Expected HostMappingsUpdated"),
        }
    }
}
