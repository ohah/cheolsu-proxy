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
