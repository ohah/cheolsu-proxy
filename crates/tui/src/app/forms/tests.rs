use super::*;

// -- UpstreamProxyField --

#[test]
fn field_next_wraps_around() {
    assert_eq!(UpstreamProxyField::Enabled.next(), UpstreamProxyField::Host);
    assert_eq!(
        UpstreamProxyField::Bypass.next(),
        UpstreamProxyField::Enabled
    );
}

#[test]
fn field_prev_wraps_around() {
    assert_eq!(
        UpstreamProxyField::Enabled.prev(),
        UpstreamProxyField::Bypass
    );
    assert_eq!(UpstreamProxyField::Host.prev(), UpstreamProxyField::Enabled);
}

#[test]
fn field_next_prev_full_cycle() {
    let mut field = UpstreamProxyField::Enabled;
    for _ in 0..UpstreamProxyField::ALL.len() {
        field = field.next();
    }
    assert_eq!(field, UpstreamProxyField::Enabled);

    for _ in 0..UpstreamProxyField::ALL.len() {
        field = field.prev();
    }
    assert_eq!(field, UpstreamProxyField::Enabled);
}

#[test]
fn field_labels_not_empty() {
    for field in UpstreamProxyField::ALL {
        assert!(!field.label().is_empty());
    }
}

// -- UpstreamProxyForm defaults --

#[test]
fn form_new_defaults() {
    let form = UpstreamProxyForm::new();
    assert!(!form.enabled);
    assert_eq!(form.field, UpstreamProxyField::Enabled);
    assert!(!form.editing);
    assert!(form.host.is_empty());
    assert_eq!(form.port, "8080");
    assert!(form.username.is_empty());
    assert!(form.password.is_empty());
    assert_eq!(form.bypass, "localhost");
}

// -- to_config --

#[test]
fn to_config_returns_none_when_disabled() {
    let mut form = UpstreamProxyForm::new();
    form.host = "proxy.example.com".to_string();
    assert!(form.to_config().is_none());
}

#[test]
fn to_config_returns_none_when_host_empty() {
    let mut form = UpstreamProxyForm::new();
    form.enabled = true;
    assert!(form.to_config().is_none());
}

#[test]
fn to_config_basic() {
    let mut form = UpstreamProxyForm::new();
    form.enabled = true;
    form.host = "proxy.example.com".to_string();
    form.port = "3128".to_string();

    let config = form.to_config().unwrap();
    assert_eq!(config.host, "proxy.example.com");
    assert_eq!(config.port, 3128);
    assert!(config.auth.is_none());
    assert_eq!(config.bypass, vec!["localhost"]);
}

#[test]
fn to_config_with_auth() {
    let mut form = UpstreamProxyForm::new();
    form.enabled = true;
    form.host = "proxy.example.com".to_string();
    form.username = "user".to_string();
    form.password = "pass".to_string();

    let config = form.to_config().unwrap();
    let auth = config.auth.unwrap();
    assert_eq!(auth.username, "user");
    assert_eq!(auth.password, "pass");
}

#[test]
fn to_config_no_auth_when_username_empty() {
    let mut form = UpstreamProxyForm::new();
    form.enabled = true;
    form.host = "proxy.example.com".to_string();
    form.password = "pass".to_string();

    let config = form.to_config().unwrap();
    assert!(config.auth.is_none());
}

#[test]
fn to_config_bypass_parsing() {
    let mut form = UpstreamProxyForm::new();
    form.enabled = true;
    form.host = "proxy.example.com".to_string();
    form.bypass = "localhost, *.internal.com, 10.0.0.1".to_string();

    let config = form.to_config().unwrap();
    assert_eq!(
        config.bypass,
        vec!["localhost", "*.internal.com", "10.0.0.1"]
    );
}

#[test]
fn to_config_bypass_empty_string() {
    let mut form = UpstreamProxyForm::new();
    form.enabled = true;
    form.host = "proxy.example.com".to_string();
    form.bypass = "".to_string();

    let config = form.to_config().unwrap();
    assert!(config.bypass.is_empty());
}

#[test]
fn to_config_bypass_trims_whitespace() {
    let mut form = UpstreamProxyForm::new();
    form.enabled = true;
    form.host = "proxy.example.com".to_string();
    form.bypass = "  localhost ,  *.test.com  ".to_string();

    let config = form.to_config().unwrap();
    assert_eq!(config.bypass, vec!["localhost", "*.test.com"]);
}

#[test]
fn to_config_invalid_port_defaults_to_8080() {
    let mut form = UpstreamProxyForm::new();
    form.enabled = true;
    form.host = "proxy.example.com".to_string();
    form.port = "not_a_number".to_string();

    let config = form.to_config().unwrap();
    assert_eq!(config.port, 8080);
}

#[test]
fn to_config_bypass_filters_empty_entries() {
    let mut form = UpstreamProxyForm::new();
    form.enabled = true;
    form.host = "proxy.example.com".to_string();
    form.bypass = "localhost,,, *.test.com, ,".to_string();

    let config = form.to_config().unwrap();
    assert_eq!(config.bypass, vec!["localhost", "*.test.com"]);
}

// -- SettingsSection --

#[test]
fn settings_section_next_prev_cycle() {
    let mut section = SettingsSection::UpstreamProxy;
    section = section.next();
    assert_eq!(section, SettingsSection::ProxyAuth);
    section = section.next();
    assert_eq!(section, SettingsSection::Throttle);
    section = section.next();
    assert_eq!(section, SettingsSection::HostMapping);
    section = section.next();
    assert_eq!(section, SettingsSection::QuickSettings);
    section = section.next();
    assert_eq!(section, SettingsSection::SslProxying);
    section = section.next();
    assert_eq!(section, SettingsSection::ClientCertificate);
    section = section.next();
    assert_eq!(section, SettingsSection::UpstreamProxy);
    section = section.prev();
    assert_eq!(section, SettingsSection::ClientCertificate);
}

// -- ProxyAuthField --

#[test]
fn proxy_auth_field_next_prev_cycle() {
    let mut field = ProxyAuthField::Enabled;
    for _ in 0..ProxyAuthField::ALL.len() {
        field = field.next();
    }
    assert_eq!(field, ProxyAuthField::Enabled);

    for _ in 0..ProxyAuthField::ALL.len() {
        field = field.prev();
    }
    assert_eq!(field, ProxyAuthField::Enabled);
}

#[test]
fn proxy_auth_field_labels_not_empty() {
    for field in ProxyAuthField::ALL {
        assert!(!field.label().is_empty());
    }
}

// -- ProxyAuthForm --

#[test]
fn proxy_auth_form_new_defaults() {
    let form = ProxyAuthForm::new();
    assert!(!form.enabled);
    assert_eq!(form.field, ProxyAuthField::Enabled);
    assert!(!form.editing);
    assert!(form.username.is_empty());
    assert!(form.password.is_empty());
}

#[test]
fn proxy_auth_form_to_config() {
    let mut form = ProxyAuthForm::new();
    form.enabled = true;
    form.username = "admin".to_string();
    form.password = "secret".to_string();
    let config = form.to_config();
    assert!(config.enabled);
    assert_eq!(config.username, "admin");
    assert_eq!(config.password, "secret");
}

#[test]
fn proxy_auth_form_to_config_disabled() {
    let form = ProxyAuthForm::new();
    let config = form.to_config();
    assert!(!config.enabled);
    assert!(config.username.is_empty());
    assert!(config.password.is_empty());
}

// -- ThrottlePresetChoice --

#[test]
fn throttle_preset_choice_labels_not_empty() {
    for preset in ThrottlePresetChoice::ALL {
        assert!(!preset.label().is_empty());
    }
}

#[test]
fn throttle_preset_choice_full_cycle() {
    let mut preset = ThrottlePresetChoice::None;
    for _ in 0..ThrottlePresetChoice::ALL.len() {
        preset = preset.next();
    }
    assert_eq!(preset, ThrottlePresetChoice::None);
}

// -- ThrottleField --

#[test]
fn throttle_field_labels_not_empty() {
    for field in ThrottleField::ALL {
        assert!(!field.label().is_empty());
    }
}

#[test]
fn throttle_field_next_prev_cycle() {
    let mut field = ThrottleField::Enabled;
    for _ in 0..ThrottleField::ALL.len() {
        field = field.next();
    }
    assert_eq!(field, ThrottleField::Enabled);

    for _ in 0..ThrottleField::ALL.len() {
        field = field.prev();
    }
    assert_eq!(field, ThrottleField::Enabled);
}

// -- ThrottleForm --

#[test]
fn throttle_form_new_defaults() {
    let form = ThrottleForm::new();
    assert!(!form.enabled);
    assert_eq!(form.preset, ThrottlePresetChoice::None);
    assert_eq!(form.field, ThrottleField::Enabled);
    assert!(!form.editing);
    assert!(form.download.is_empty());
    assert!(form.upload.is_empty());
    assert_eq!(form.latency, "0");
}

#[test]
fn throttle_form_to_config_disabled_returns_none() {
    let form = ThrottleForm::new();
    assert!(form.to_config().is_none());
}

#[test]
fn throttle_form_to_config_enabled_none_preset_returns_none() {
    let mut form = ThrottleForm::new();
    form.enabled = true;
    form.preset = ThrottlePresetChoice::None;
    assert!(form.to_config().is_none());
}

#[test]
fn throttle_form_to_config_gprs_matches_core_preset() {
    let mut form = ThrottleForm::new();
    form.enabled = true;
    form.preset = ThrottlePresetChoice::Gprs;
    let config = form.to_config().unwrap();
    let core_config = proxy_daemon::ThrottlePreset::Gprs.to_config();
    assert_eq!(config.download_rate, core_config.download_rate);
    assert_eq!(config.upload_rate, core_config.upload_rate);
    assert_eq!(config.latency_ms, core_config.latency_ms);
}

#[test]
fn throttle_form_to_config_all_presets_match_core() {
    let pairs: Vec<(ThrottlePresetChoice, proxy_daemon::ThrottlePreset)> = vec![
        (
            ThrottlePresetChoice::Gprs,
            proxy_daemon::ThrottlePreset::Gprs,
        ),
        (
            ThrottlePresetChoice::Slow3G,
            proxy_daemon::ThrottlePreset::Slow3G,
        ),
        (
            ThrottlePresetChoice::Fast3G,
            proxy_daemon::ThrottlePreset::Fast3G,
        ),
        (ThrottlePresetChoice::Lte, proxy_daemon::ThrottlePreset::Lte),
        (
            ThrottlePresetChoice::Wifi,
            proxy_daemon::ThrottlePreset::Wifi,
        ),
    ];
    for (tui_preset, core_preset) in pairs {
        let mut form = ThrottleForm::new();
        form.enabled = true;
        form.preset = tui_preset;
        let config = form.to_config().unwrap();
        let core_config = core_preset.to_config();
        assert_eq!(
            config.download_rate, core_config.download_rate,
            "download mismatch for {:?}",
            tui_preset
        );
        assert_eq!(
            config.upload_rate, core_config.upload_rate,
            "upload mismatch for {:?}",
            tui_preset
        );
        assert_eq!(
            config.latency_ms, core_config.latency_ms,
            "latency mismatch for {:?}",
            tui_preset
        );
    }
}

#[test]
fn throttle_form_to_config_custom() {
    let mut form = ThrottleForm::new();
    form.enabled = true;
    form.preset = ThrottlePresetChoice::Custom;
    form.download = "1024".to_string();
    form.upload = "512".to_string();
    form.latency = "100".to_string();

    let config = form.to_config().unwrap();
    assert!(config.enabled);
    assert_eq!(config.download_rate, Some(1024 * 1024));
    assert_eq!(config.upload_rate, Some(512 * 1024));
    assert_eq!(config.latency_ms, 100);
}

#[test]
fn throttle_form_to_config_custom_zero_rate_is_none() {
    let mut form = ThrottleForm::new();
    form.enabled = true;
    form.preset = ThrottlePresetChoice::Custom;
    form.download = "0".to_string();
    form.upload = "0".to_string();
    form.latency = "50".to_string();

    let config = form.to_config().unwrap();
    assert!(config.download_rate.is_none());
    assert!(config.upload_rate.is_none());
    assert_eq!(config.latency_ms, 50);
}

#[test]
fn throttle_form_to_config_custom_empty_strings() {
    let mut form = ThrottleForm::new();
    form.enabled = true;
    form.preset = ThrottlePresetChoice::Custom;
    form.download = "".to_string();
    form.upload = "".to_string();
    form.latency = "".to_string();

    let config = form.to_config().unwrap();
    assert!(config.download_rate.is_none());
    assert!(config.upload_rate.is_none());
    assert_eq!(config.latency_ms, 0);
}

#[test]
fn throttle_form_to_config_custom_invalid_input() {
    let mut form = ThrottleForm::new();
    form.enabled = true;
    form.preset = ThrottlePresetChoice::Custom;
    form.download = "abc".to_string();
    form.upload = "xyz".to_string();
    form.latency = "not_a_number".to_string();

    let config = form.to_config().unwrap();
    assert!(config.download_rate.is_none());
    assert!(config.upload_rate.is_none());
    assert_eq!(config.latency_ms, 0);
}

// ── RuleFormField ──

#[test]
fn rule_form_field_next_block() {
    // Block: Name → Pattern → Method → ActionType → StatusCode → Body → Name
    let path = [
        RuleFormField::Name,
        RuleFormField::Pattern,
        RuleFormField::Method,
        RuleFormField::ActionType,
        RuleFormField::StatusCode,
        RuleFormField::Body,
    ];
    let mut field = RuleFormField::Name;
    for expected in &path[1..] {
        field = field.next(ActionType::Block);
        assert_eq!(field, *expected);
    }
    // Body → Name (loop back)
    assert_eq!(
        RuleFormField::Body.next(ActionType::Block),
        RuleFormField::Name
    );
}

#[test]
fn rule_form_field_next_map_remote() {
    // MapRemote: ActionType → TargetUrl → Name
    assert_eq!(
        RuleFormField::ActionType.next(ActionType::MapRemote),
        RuleFormField::TargetUrl
    );
    assert_eq!(
        RuleFormField::TargetUrl.next(ActionType::MapRemote),
        RuleFormField::Name
    );
}

#[test]
fn rule_form_field_next_map_local() {
    // MapLocal: ActionType → FilePath → StatusCode → Body → Name
    assert_eq!(
        RuleFormField::ActionType.next(ActionType::MapLocal),
        RuleFormField::FilePath
    );
    assert_eq!(
        RuleFormField::FilePath.next(ActionType::MapLocal),
        RuleFormField::StatusCode
    );
}

#[test]
fn rule_form_field_next_modify_request() {
    // ModifyRequest: ActionType → Body → Name
    assert_eq!(
        RuleFormField::ActionType.next(ActionType::ModifyRequest),
        RuleFormField::Body
    );
}

#[test]
fn rule_form_field_prev_block() {
    // Block: Name ← Body ← StatusCode ← ActionType ← Method ← Pattern ← Name
    assert_eq!(
        RuleFormField::Name.prev(ActionType::Block),
        RuleFormField::Body
    );
    assert_eq!(
        RuleFormField::Body.prev(ActionType::Block),
        RuleFormField::StatusCode
    );
    assert_eq!(
        RuleFormField::StatusCode.prev(ActionType::Block),
        RuleFormField::ActionType
    );
}

#[test]
fn rule_form_field_prev_map_remote() {
    assert_eq!(
        RuleFormField::Name.prev(ActionType::MapRemote),
        RuleFormField::TargetUrl
    );
    assert_eq!(
        RuleFormField::TargetUrl.prev(ActionType::MapRemote),
        RuleFormField::ActionType
    );
}

#[test]
fn rule_form_field_prev_map_local() {
    assert_eq!(
        RuleFormField::Name.prev(ActionType::MapLocal),
        RuleFormField::StatusCode
    );
    assert_eq!(
        RuleFormField::StatusCode.prev(ActionType::MapLocal),
        RuleFormField::FilePath
    );
}

// ── ActionType ──

#[test]
fn action_type_all_labels_not_empty() {
    for action in ActionType::ALL {
        assert!(!action.label().is_empty());
    }
}

#[test]
fn action_type_full_cycle() {
    let mut action = ActionType::Block;
    for _ in 0..ActionType::ALL.len() {
        action = action.next();
    }
    assert_eq!(action, ActionType::Block);

    for _ in 0..ActionType::ALL.len() {
        action = action.prev();
    }
    assert_eq!(action, ActionType::Block);
}

// ── RuleForm ──

#[test]
fn rule_form_new_defaults() {
    let form = RuleForm::new();
    assert_eq!(form.field, RuleFormField::Name);
    assert!(form.name.is_empty());
    assert!(form.pattern.is_empty());
    assert!(form.method.is_none());
    assert_eq!(form.action_type, ActionType::Block);
    assert_eq!(form.status_code, "403");
}

#[test]
fn rule_form_to_rule_none_when_pattern_empty() {
    let form = RuleForm::new();
    assert!(form.to_rule().is_none());
}

#[test]
fn rule_form_to_rule_block() {
    let mut form = RuleForm::new();
    form.pattern = "*.example.com".to_string();
    form.name = "Block example".to_string();
    form.status_code = "404".to_string();
    form.body = "Not Found".to_string();

    let rule = form.to_rule().unwrap();
    assert_eq!(rule.name, "Block example");
    assert_eq!(rule.pattern, "*.example.com");
    assert!(rule.enabled);
    match &rule.action {
        proxy_daemon::InterceptAction::Block { status_code, body } => {
            assert_eq!(*status_code, 404);
            assert_eq!(body, "Not Found");
        }
        _ => panic!("Expected Block action"),
    }
}

#[test]
fn rule_form_to_rule_uses_pattern_as_name_when_name_empty() {
    let mut form = RuleForm::new();
    form.pattern = "*.test.com".to_string();

    let rule = form.to_rule().unwrap();
    assert_eq!(rule.name, "*.test.com");
}

#[test]
fn rule_form_to_rule_invalid_status_defaults_to_403() {
    let mut form = RuleForm::new();
    form.pattern = "test".to_string();
    form.status_code = "invalid".to_string();

    let rule = form.to_rule().unwrap();
    match &rule.action {
        proxy_daemon::InterceptAction::Block { status_code, .. } => {
            assert_eq!(*status_code, 403);
        }
        _ => panic!("Expected Block action"),
    }
}

#[test]
fn rule_form_to_rule_map_remote() {
    let mut form = RuleForm::new();
    form.pattern = "*.api.com".to_string();
    form.action_type = ActionType::MapRemote;
    form.target_url = "http://localhost:3000".to_string();

    let rule = form.to_rule().unwrap();
    match &rule.action {
        proxy_daemon::InterceptAction::MapRemote {
            target_url,
            preserve_path,
        } => {
            assert_eq!(target_url, "http://localhost:3000");
            assert!(*preserve_path);
        }
        _ => panic!("Expected MapRemote action"),
    }
}

#[test]
fn rule_form_to_rule_map_local() {
    let mut form = RuleForm::new();
    form.pattern = "*.css".to_string();
    form.action_type = ActionType::MapLocal;
    form.file_path = "/tmp/local.css".to_string();
    form.status_code = "200".to_string();

    let rule = form.to_rule().unwrap();
    match &rule.action {
        proxy_daemon::InterceptAction::MapLocal {
            file_path,
            status_code,
            ..
        } => {
            assert_eq!(file_path, "/tmp/local.css");
            assert_eq!(*status_code, 200);
        }
        _ => panic!("Expected MapLocal action"),
    }
}

#[test]
fn rule_form_to_rule_modify_request_empty_body() {
    let mut form = RuleForm::new();
    form.pattern = "test".to_string();
    form.action_type = ActionType::ModifyRequest;
    form.body = "".to_string();

    let rule = form.to_rule().unwrap();
    match &rule.action {
        proxy_daemon::InterceptAction::ModifyRequest { set_body, .. } => {
            assert!(set_body.is_none());
        }
        _ => panic!("Expected ModifyRequest action"),
    }
}

#[test]
fn rule_form_to_rule_modify_request_with_body() {
    let mut form = RuleForm::new();
    form.pattern = "test".to_string();
    form.action_type = ActionType::ModifyRequest;
    form.body = r#"{"injected":true}"#.to_string();

    let rule = form.to_rule().unwrap();
    match &rule.action {
        proxy_daemon::InterceptAction::ModifyRequest { set_body, .. } => {
            assert_eq!(set_body.as_deref(), Some(r#"{"injected":true}"#));
        }
        _ => panic!("Expected ModifyRequest action"),
    }
}

#[test]
fn rule_form_to_rule_modify_response() {
    let mut form = RuleForm::new();
    form.pattern = "test".to_string();
    form.action_type = ActionType::ModifyResponse;
    form.status_code = "201".to_string();
    form.body = "created".to_string();

    let rule = form.to_rule().unwrap();
    match &rule.action {
        proxy_daemon::InterceptAction::ModifyResponse {
            set_status,
            set_body,
            ..
        } => {
            assert_eq!(*set_status, Some(201));
            assert_eq!(set_body.as_deref(), Some("created"));
        }
        _ => panic!("Expected ModifyResponse action"),
    }
}

// ── HostMappingForm ──

#[test]
fn host_mapping_field_full_cycle() {
    let mut field = HostMappingField::SourceHost;
    for _ in 0..HostMappingField::ALL.len() {
        field = field.next();
    }
    assert_eq!(field, HostMappingField::SourceHost);
}

#[test]
fn host_mapping_field_labels_not_empty() {
    for field in HostMappingField::ALL {
        assert!(!field.label().is_empty());
    }
}

#[test]
fn host_mapping_form_new_defaults() {
    let form = HostMappingForm::new();
    assert_eq!(form.field, HostMappingField::SourceHost);
    assert!(form.source_host.is_empty());
    assert!(form.source_port.is_empty());
    assert!(form.target_host.is_empty());
    assert!(form.target_port.is_empty());
}

#[test]
fn host_mapping_to_mapping_none_when_source_empty() {
    let form = HostMappingForm::new();
    assert!(form.to_mapping().is_none());
}

#[test]
fn host_mapping_to_mapping_none_when_target_empty() {
    let mut form = HostMappingForm::new();
    form.source_host = "example.com".to_string();
    assert!(form.to_mapping().is_none());
}

#[test]
fn host_mapping_to_mapping_basic() {
    let mut form = HostMappingForm::new();
    form.source_host = "api.example.com".to_string();
    form.target_host = "localhost".to_string();

    let mapping = form.to_mapping().unwrap();
    assert_eq!(mapping.source_host, "api.example.com");
    assert_eq!(mapping.target_host, "localhost");
    assert!(mapping.source_port.is_none());
    assert!(mapping.target_port.is_none());
    assert!(mapping.enabled);
    assert!(mapping.id.starts_with("hm_"));
}

#[test]
fn host_mapping_to_mapping_with_ports() {
    let mut form = HostMappingForm::new();
    form.source_host = "api.com".to_string();
    form.source_port = "443".to_string();
    form.target_host = "localhost".to_string();
    form.target_port = "3000".to_string();

    let mapping = form.to_mapping().unwrap();
    assert_eq!(mapping.source_port, Some(443));
    assert_eq!(mapping.target_port, Some(3000));
}

#[test]
fn host_mapping_to_mapping_invalid_port() {
    let mut form = HostMappingForm::new();
    form.source_host = "api.com".to_string();
    form.source_port = "invalid".to_string();
    form.target_host = "localhost".to_string();

    let mapping = form.to_mapping().unwrap();
    assert!(mapping.source_port.is_none());
}

#[test]
fn host_mapping_clear() {
    let mut form = HostMappingForm::new();
    form.source_host = "api.com".to_string();
    form.target_host = "localhost".to_string();
    form.field = HostMappingField::TargetHost;

    form.clear();
    assert!(form.source_host.is_empty());
    assert!(form.target_host.is_empty());
    assert_eq!(form.field, HostMappingField::SourceHost);
}

// ── ClientCertForm ──

#[test]
fn client_cert_field_full_cycle() {
    let mut field = ClientCertField::Enabled;
    for _ in 0..ClientCertField::ALL.len() {
        field = field.next();
    }
    assert_eq!(field, ClientCertField::Enabled);
}

#[test]
fn client_cert_field_labels_not_empty() {
    for field in ClientCertField::ALL {
        assert!(!field.label().is_empty());
    }
}

#[test]
fn client_cert_form_new_defaults() {
    let form = ClientCertForm::new();
    assert!(!form.enabled);
    assert!(form.cert_path.is_empty());
    assert!(form.key_path.is_empty());
    assert_eq!(form.field, ClientCertField::Enabled);
    assert!(!form.editing);
}

#[test]
fn client_cert_to_config_none_when_disabled() {
    let mut form = ClientCertForm::new();
    form.cert_path = "/cert.pem".to_string();
    form.key_path = "/key.pem".to_string();
    assert!(form.to_config().is_none());
}

#[test]
fn client_cert_to_config_none_when_cert_empty() {
    let mut form = ClientCertForm::new();
    form.enabled = true;
    form.key_path = "/key.pem".to_string();
    assert!(form.to_config().is_none());
}

#[test]
fn client_cert_to_config_none_when_key_empty() {
    let mut form = ClientCertForm::new();
    form.enabled = true;
    form.cert_path = "/cert.pem".to_string();
    assert!(form.to_config().is_none());
}

#[test]
fn client_cert_to_config_valid() {
    let mut form = ClientCertForm::new();
    form.enabled = true;
    form.cert_path = "/cert.pem".to_string();
    form.key_path = "/key.pem".to_string();

    let config = form.to_config().unwrap();
    assert!(config.enabled);
    assert_eq!(config.cert_path, "/cert.pem");
    assert_eq!(config.key_path, "/key.pem");
    assert!(config.domain_certs.is_empty());
}

// ── SslProxyingAddForm ──

#[test]
fn ssl_proxying_add_form_new_defaults() {
    let form = SslProxyingAddForm::new();
    assert!(form.pattern.is_empty());
}

#[test]
fn ssl_proxying_to_entry_none_when_empty() {
    let form = SslProxyingAddForm::new();
    assert!(form.to_entry().is_none());
}

#[test]
fn ssl_proxying_to_entry_valid() {
    let mut form = SslProxyingAddForm::new();
    form.pattern = "*.example.com".to_string();

    let entry = form.to_entry().unwrap();
    assert_eq!(entry.pattern, "*.example.com");
    assert!(entry.enabled);
}

// ── QuickSettingsForm ──

#[test]
fn quick_settings_field_full_cycle() {
    let mut field = QuickSettingsField::NoCaching;
    for _ in 0..QuickSettingsField::ALL.len() {
        field = field.next();
    }
    assert_eq!(field, QuickSettingsField::NoCaching);
}

#[test]
fn quick_settings_field_labels_not_empty() {
    for field in QuickSettingsField::ALL {
        assert!(!field.label().is_empty());
    }
}

#[test]
fn quick_settings_form_new_defaults() {
    let form = QuickSettingsForm::new();
    assert!(!form.no_caching);
    assert!(!form.block_cookies);
    assert!(!form.no_gzip);
    assert_eq!(form.field, QuickSettingsField::NoCaching);
}
