use proxy_v2_models::{RequestInfo, WsConnectionEvent, WsMessageInfo};
use proxyapi_v2::throttle::ThrottleConfig;
use proxyapi_v2::upstream_proxy::UpstreamProxyConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 클라이언트 인증서 설정 (mTLS)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientCertConfig {
    /// 클라이언트 인증서 파일 경로 (.pem, .crt)
    pub cert_path: String,
    /// 클라이언트 키 파일 경로 (.pem, .key)
    pub key_path: String,
    /// 활성화 여부
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum DaemonMessage {
    #[serde(rename = "event")]
    Event { data: RequestInfo },
    #[serde(rename = "status")]
    Status { running: bool, port: u16 },
    #[serde(rename = "intercept_rules_updated")]
    InterceptRulesUpdated { rules: Vec<InterceptRule> },
    #[serde(rename = "host_mappings_updated")]
    HostMappingsUpdated { mappings: Vec<HostMapping> },
    #[serde(rename = "ws_message")]
    WsMessage { data: WsMessageInfo },
    #[serde(rename = "ws_connection")]
    WsConnection { data: WsConnectionEvent },
    #[serde(rename = "ws_inject_result")]
    WsInjectResult {
        success: bool,
        error: Option<String>,
    },
    /// 스크립트 로드/언로드 결과
    #[serde(rename = "script_result")]
    ScriptResult {
        success: bool,
        error: Option<String>,
    },
    /// 스크립트 console.log 등의 로그
    #[serde(rename = "script_log")]
    ScriptLog { level: String, message: String },
    /// 스크립트 상태 변경 (로드/언로드/리로드)
    #[serde(rename = "script_status")]
    ScriptStatus {
        active: bool,
        path: Option<String>,
        message: String,
    },
    /// A breakpoint was hit (request or response paused)
    #[serde(rename = "breakpoint_hit")]
    BreakpointHit {
        id: String,
        transaction_id: String,
        phase: BreakpointPhase,
        data: BreakpointData,
    },
    /// Breakpoint rules updated
    #[serde(rename = "breakpoint_rules_updated")]
    BreakpointRulesUpdated { rules: Vec<BreakpointRule> },
    #[serde(rename = "session_saved")]
    SessionSaved {
        path: String,
        transaction_count: usize,
    },
    #[serde(rename = "session_loaded")]
    SessionLoaded {
        path: String,
        transaction_count: usize,
    },
    /// SSL Proxying 화이트리스트 업데이트됨
    #[serde(rename = "ssl_proxying_list_updated")]
    SslProxyingListUpdated { entries: Vec<SslProxyingEntry> },
    /// 클라이언트 인증서 설정 업데이트됨
    #[serde(rename = "client_certificate_updated")]
    ClientCertificateUpdated { config: Option<ClientCertConfig> },
    /// 헬스체크 결과
    #[serde(rename = "health_check_result")]
    HealthCheckResult {
        uptime_secs: u64,
        active_connections: u32,
        total_transactions: u64,
    },
    /// 데몬 연결이 끊어졌음을 알리는 메시지
    #[serde(rename = "disconnected")]
    Disconnected { reason: String },
    /// 데몬에 재연결되었음을 알리는 메시지
    #[serde(rename = "reconnected")]
    Reconnected,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd")]
pub enum ClientCommand {
    #[serde(rename = "subscribe")]
    Subscribe,
    #[serde(rename = "update_intercept_rules")]
    UpdateInterceptRules { rules: Vec<InterceptRule> },
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "ws_inject")]
    WsInject {
        connection_id: String,
        direction: String,
        payload: String,
        is_binary: bool,
    },
    #[serde(rename = "update_upstream_proxy")]
    UpdateUpstreamProxy { config: Option<UpstreamProxyConfig> },
    #[serde(rename = "update_server_replay")]
    UpdateServerReplay { entries: Vec<ServerReplayEntry> },
    /// 스크립트 로드 (파일 경로 또는 인라인 코드)
    #[serde(rename = "load_script")]
    LoadScript {
        /// 스크립트 파일 경로 (path와 code 중 하나 필수)
        #[serde(default)]
        path: Option<String>,
        /// 인라인 스크립트 코드
        #[serde(default)]
        code: Option<String>,
    },
    /// 스크립트 언로드
    #[serde(rename = "unload_script")]
    UnloadScript,
    /// 스로틀링 설정 업데이트
    #[serde(rename = "update_throttle")]
    UpdateThrottle { config: Option<ThrottleConfig> },
    /// Update breakpoint rules
    #[serde(rename = "update_breakpoint_rules")]
    UpdateBreakpointRules { rules: Vec<BreakpointRule> },
    /// Resolve a pending breakpoint
    #[serde(rename = "resolve_breakpoint")]
    ResolveBreakpoint {
        id: String,
        action: BreakpointAction,
    },
    #[serde(rename = "save_session")]
    SaveSession {
        path: String,
        #[serde(default)]
        filter: Option<String>,
    },
    #[serde(rename = "load_session")]
    LoadSession { path: String },
    #[serde(rename = "update_host_mappings")]
    UpdateHostMappings { mappings: Vec<HostMapping> },
    /// 빠른 설정 업데이트 (No Caching, Block Cookies, No Gzip)
    #[serde(rename = "update_quick_settings")]
    UpdateQuickSettings {
        no_caching: bool,
        block_cookies: bool,
        #[serde(default)]
        no_gzip: bool,
    },
    /// SSL Proxying 화이트리스트 업데이트
    #[serde(rename = "update_ssl_proxying_list")]
    UpdateSslProxyingList { entries: Vec<SslProxyingEntry> },
    /// 프록시 인증 설정 업데이트
    #[serde(rename = "update_proxy_auth")]
    UpdateProxyAuth { config: ProxyAuthConfig },
    /// 클라이언트 인증서 설정 업데이트 (mTLS)
    #[serde(rename = "update_client_certificate")]
    UpdateClientCertificate { config: Option<ClientCertConfig> },
    /// 헬스체크 요청
    #[serde(rename = "health_check")]
    HealthCheck,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProxyLockInfo {
    pub pid: u32,
    pub port: u16,
    pub uds_path: String,
}

/// 인터셉트 규칙 정의
/// `pattern`은 와일드카드 URL 패턴 (예: `*.example.com/api/*`)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterceptRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub pattern: String,
    #[serde(default)]
    pub method: Option<String>,
    pub action: InterceptAction,
}

/// Rewrite 대상 열거형
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum RewriteTarget {
    #[serde(rename = "request_header")]
    RequestHeader,
    #[serde(rename = "response_header")]
    ResponseHeader,
    #[serde(rename = "request_body")]
    RequestBody,
    #[serde(rename = "response_body")]
    ResponseBody,
}

/// 인터셉트 동작
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum InterceptAction {
    /// 요청을 차단하고 지정된 응답을 반환
    #[serde(rename = "block")]
    Block {
        #[serde(default = "default_block_status")]
        status_code: u16,
        #[serde(default)]
        body: String,
    },
    /// 요청 헤더/바디를 수정하여 전달
    #[serde(rename = "modify_request")]
    ModifyRequest {
        #[serde(default)]
        add_headers: HashMap<String, String>,
        #[serde(default)]
        remove_headers: Vec<String>,
        #[serde(default)]
        set_body: Option<String>,
    },
    /// 응답 헤더/바디/상태코드를 수정
    #[serde(rename = "modify_response")]
    ModifyResponse {
        #[serde(default)]
        set_status: Option<u16>,
        #[serde(default)]
        add_headers: HashMap<String, String>,
        #[serde(default)]
        remove_headers: Vec<String>,
        #[serde(default)]
        set_body: Option<String>,
    },
    /// 요청을 로컬 파일의 내용으로 응답 (Map Local)
    #[serde(rename = "map_local")]
    MapLocal {
        /// 로컬 파일 경로
        file_path: String,
        /// 응답 상태 코드 (기본: 200)
        #[serde(default = "default_ok_status")]
        status_code: u16,
        /// 추가 응답 헤더
        #[serde(default)]
        headers: HashMap<String, String>,
    },
    /// 요청을 다른 URL로 리다이렉트 (Map Remote)
    #[serde(rename = "map_remote")]
    MapRemote {
        /// 대상 URL (예: http://localhost:3000)
        target_url: String,
        /// 원본 경로 유지 여부 (true면 원본 path를 target_url에 붙임)
        #[serde(default = "default_true")]
        preserve_path: bool,
    },
    /// 정규식을 사용하여 요청/응답의 헤더 또는 바디를 치환 (Rewrite)
    #[serde(rename = "rewrite")]
    Rewrite {
        /// 적용 대상
        target: RewriteTarget,
        /// 매칭할 정규식 패턴
        match_pattern: String,
        /// 치환 문자열 ($1, $2 등 캡처 그룹 지원)
        replace_with: String,
    },
}

/// Host mapping entry for DNS spoofing / remote host mapping.
/// Maps requests from source host to a different target host/IP,
/// allowing testing against staging/dev servers without modifying hosts file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HostMapping {
    pub id: String,
    /// Source host pattern, supports wildcards (e.g., "*.api.example.com")
    pub source_host: String,
    /// Source port filter (None = any port)
    pub source_port: Option<u16>,
    /// Target host (IP address or domain name)
    pub target_host: String,
    /// Target port (None = keep original port)
    pub target_port: Option<u16>,
    pub enabled: bool,
}

impl std::fmt::Display for HostMapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.enabled { "enabled" } else { "disabled" };
        let src_port = self
            .source_port
            .map(|p| format!(":{}", p))
            .unwrap_or_default();
        let tgt_port = self
            .target_port
            .map(|p| format!(":{}", p))
            .unwrap_or_default();
        write!(
            f,
            "[{}] {}{} -> {}{} [{}]",
            self.id, self.source_host, src_port, self.target_host, tgt_port, status
        )
    }
}

/// 서버 리플레이 엔트리: 캡처된 응답을 저장하여 동일 요청 시 재사용
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerReplayEntry {
    pub id: String,
    pub method: String,
    pub url: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// Breakpoint rule: pause matching requests/responses for manual editing.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BreakpointRule {
    pub id: String,
    pub pattern: String,
    #[serde(default)]
    pub break_on_request: bool,
    #[serde(default)]
    pub break_on_response: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Action to take on a paused breakpoint.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum BreakpointAction {
    /// Forward the request/response as-is.
    #[serde(rename = "forward")]
    Forward,
    /// Modify and then forward.
    #[serde(rename = "modify_and_forward")]
    ModifyAndForward {
        #[serde(default)]
        headers: Option<HashMap<String, String>>,
        #[serde(default)]
        body: Option<String>,
        #[serde(default)]
        status: Option<u16>,
    },
    /// Drop the request (close connection).
    #[serde(rename = "drop")]
    Drop,
    /// Abort with an error response.
    #[serde(rename = "abort")]
    Abort,
}

/// Phase at which a breakpoint was hit.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum BreakpointPhase {
    #[serde(rename = "request")]
    Request,
    #[serde(rename = "response")]
    Response,
}

/// Data snapshot for a paused breakpoint.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BreakpointData {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub status: Option<u16>,
}

/// SSL Proxying 화이트리스트 엔트리
/// 지정된 도메인만 HTTPS 트래픽을 인터셉트(MITM)하고, 나머지는 TLS Passthrough
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SslProxyingEntry {
    /// 도메인 패턴 (예: "example.com", "*.example.com", "example.com:443")
    pub pattern: String,
    pub enabled: bool,
}

/// 프록시 서버 자체의 인증 설정
/// 활성화 시, 클라이언트가 Proxy-Authorization 헤더로 Basic 인증을 해야만 프록시를 사용할 수 있음
#[derive(Serialize, Deserialize, Clone)]
pub struct ProxyAuthConfig {
    pub enabled: bool,
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for ProxyAuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyAuthConfig")
            .field("enabled", &self.enabled)
            .field("username", &self.username)
            .field("password", &"****")
            .finish()
    }
}

impl ProxyAuthConfig {
    /// Basic 인증 헤더 값을 생성합니다.
    pub fn expected_basic_header(&self) -> String {
        use base64::Engine;
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        format!("Basic {}", encoded)
    }

    /// Proxy-Authorization 헤더 값을 검증합니다.
    pub fn validate_proxy_auth(&self, auth_header: Option<&str>) -> bool {
        if !self.enabled {
            return true;
        }
        if self.username.is_empty() {
            return true;
        }
        match auth_header {
            Some(header) => header == self.expected_basic_header(),
            None => false,
        }
    }
}

fn default_block_status() -> u16 {
    403
}

fn default_ok_status() -> u16 {
    200
}

fn default_true() -> bool {
    true
}

impl std::fmt::Display for BreakpointRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.enabled { "enabled" } else { "disabled" };
        let phases = match (self.break_on_request, self.break_on_response) {
            (true, true) => "req+res",
            (true, false) => "req",
            (false, true) => "res",
            (false, false) => "none",
        };
        write!(
            f,
            "[{}] {} (break on: {}) [{}]",
            self.id, self.pattern, phases, status
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // 역직렬화 검증
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
    fn test_ssl_proxying_entry_serde_roundtrip() {
        let entry = SslProxyingEntry {
            pattern: "*.example.com".to_string(),
            enabled: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: SslProxyingEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.pattern, "*.example.com");
        assert!(deserialized.enabled);
    }

    #[test]
    fn test_update_ssl_proxying_list_command_serialize() {
        let cmd = ClientCommand::UpdateSslProxyingList {
            entries: vec![
                SslProxyingEntry {
                    pattern: "example.com".to_string(),
                    enabled: true,
                },
                SslProxyingEntry {
                    pattern: "*.api.io:8443".to_string(),
                    enabled: false,
                },
            ],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_ssl_proxying_list"));
        assert!(json.contains("example.com"));

        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateSslProxyingList { entries } => {
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].pattern, "example.com");
                assert!(entries[0].enabled);
                assert_eq!(entries[1].pattern, "*.api.io:8443");
                assert!(!entries[1].enabled);
            }
            _ => panic!("Expected UpdateSslProxyingList"),
        }
    }

    #[test]
    fn test_ssl_proxying_list_updated_message_serialize() {
        let msg = DaemonMessage::SslProxyingListUpdated {
            entries: vec![SslProxyingEntry {
                pattern: "*.example.com".to_string(),
                enabled: true,
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("ssl_proxying_list_updated"));

        let deserialized: DaemonMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            DaemonMessage::SslProxyingListUpdated { entries } => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].pattern, "*.example.com");
            }
            _ => panic!("Expected SslProxyingListUpdated"),
        }
    }

    #[test]
    fn test_proxy_auth_config_debug_masks_password() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "super_secret_password".to_string(),
        };
        let debug_output = format!("{:?}", config);
        assert!(
            !debug_output.contains("super_secret_password"),
            "Debug output must not contain the actual password"
        );
        assert!(
            debug_output.contains("****"),
            "Debug output must mask the password with ****"
        );
        assert!(debug_output.contains("admin"));
    }

    #[test]
    fn test_proxy_auth_config_serde_roundtrip() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret123".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ProxyAuthConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.enabled);
        assert_eq!(deserialized.username, "admin");
        assert_eq!(deserialized.password, "secret123");
    }

    #[test]
    fn test_proxy_auth_expected_basic_header() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret".to_string(),
        };
        let header = config.expected_basic_header();
        // base64("admin:secret") = "YWRtaW46c2VjcmV0"
        assert_eq!(header, "Basic YWRtaW46c2VjcmV0");
    }

    #[test]
    fn test_proxy_auth_validate_success() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret".to_string(),
        };
        assert!(config.validate_proxy_auth(Some("Basic YWRtaW46c2VjcmV0")));
    }

    #[test]
    fn test_proxy_auth_validate_failure_wrong_credentials() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret".to_string(),
        };
        assert!(!config.validate_proxy_auth(Some("Basic d3Jvbmc6Y3JlZHM=")));
    }

    #[test]
    fn test_proxy_auth_validate_failure_no_header() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret".to_string(),
        };
        assert!(!config.validate_proxy_auth(None));
    }

    #[test]
    fn test_proxy_auth_validate_disabled_always_passes() {
        let config = ProxyAuthConfig {
            enabled: false,
            username: "admin".to_string(),
            password: "secret".to_string(),
        };
        assert!(config.validate_proxy_auth(None));
        assert!(config.validate_proxy_auth(Some("garbage")));
    }

    #[test]
    fn test_proxy_auth_validate_empty_username_always_passes() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: String::new(),
            password: "secret".to_string(),
        };
        assert!(config.validate_proxy_auth(None));
    }

    #[test]
    fn test_update_proxy_auth_command_serialize() {
        let cmd = ClientCommand::UpdateProxyAuth {
            config: ProxyAuthConfig {
                enabled: true,
                username: "user".to_string(),
                password: "pass".to_string(),
            },
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_proxy_auth"));
        assert!(json.contains("user"));

        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateProxyAuth { config } => {
                assert!(config.enabled);
                assert_eq!(config.username, "user");
                assert_eq!(config.password, "pass");
            }
            _ => panic!("Expected UpdateProxyAuth"),
        }
    }

    #[test]
    fn test_client_cert_config_serde_roundtrip() {
        let config = ClientCertConfig {
            cert_path: "/path/to/cert.pem".to_string(),
            key_path: "/path/to/key.pem".to_string(),
            enabled: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ClientCertConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cert_path, "/path/to/cert.pem");
        assert_eq!(deserialized.key_path, "/path/to/key.pem");
        assert!(deserialized.enabled);
    }

    #[test]
    fn test_update_client_certificate_command_serialize() {
        let cmd = ClientCommand::UpdateClientCertificate {
            config: Some(ClientCertConfig {
                cert_path: "/tmp/client.crt".to_string(),
                key_path: "/tmp/client.key".to_string(),
                enabled: true,
            }),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_client_certificate"));
        assert!(json.contains("/tmp/client.crt"));

        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateClientCertificate { config } => {
                let config = config.unwrap();
                assert_eq!(config.cert_path, "/tmp/client.crt");
                assert_eq!(config.key_path, "/tmp/client.key");
                assert!(config.enabled);
            }
            _ => panic!("Expected UpdateClientCertificate"),
        }
    }

    #[test]
    fn test_update_client_certificate_command_none() {
        let cmd = ClientCommand::UpdateClientCertificate { config: None };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_client_certificate"));

        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateClientCertificate { config } => {
                assert!(config.is_none());
            }
            _ => panic!("Expected UpdateClientCertificate"),
        }
    }

    #[test]
    fn test_client_certificate_updated_message_serialize() {
        let msg = DaemonMessage::ClientCertificateUpdated {
            config: Some(ClientCertConfig {
                cert_path: "/path/cert.pem".to_string(),
                key_path: "/path/key.pem".to_string(),
                enabled: true,
            }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("client_certificate_updated"));

        let deserialized: DaemonMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            DaemonMessage::ClientCertificateUpdated { config } => {
                let config = config.unwrap();
                assert_eq!(config.cert_path, "/path/cert.pem");
            }
            _ => panic!("Expected ClientCertificateUpdated"),
        }
    }

    // --- Disconnected / Reconnected DaemonMessage 직렬화/역직렬화 테스트 ---

    /// Disconnected 메시지 직렬화 검증
    #[test]
    fn test_daemon_message_disconnected_serialization() {
        let msg = DaemonMessage::Disconnected {
            reason: "daemon process killed".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "disconnected");
        assert_eq!(parsed["reason"], "daemon process killed");
    }

    /// Disconnected 메시지 역직렬화 검증
    #[test]
    fn test_daemon_message_disconnected_deserialization() {
        let json = r#"{"type":"disconnected","reason":"connection lost"}"#;
        let msg: DaemonMessage = serde_json::from_str(json).unwrap();
        match msg {
            DaemonMessage::Disconnected { reason } => {
                assert_eq!(reason, "connection lost");
            }
            _ => panic!("Expected Disconnected"),
        }
    }

    /// Disconnected 메시지 roundtrip 검증
    #[test]
    fn test_daemon_message_disconnected_roundtrip() {
        let msg = DaemonMessage::Disconnected {
            reason: "프로세스 강제 종료".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: DaemonMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            DaemonMessage::Disconnected { reason } => {
                assert_eq!(reason, "프로세스 강제 종료");
            }
            _ => panic!("Expected Disconnected"),
        }
    }

    /// Reconnected 메시지 직렬화 검증
    #[test]
    fn test_daemon_message_reconnected_serialization() {
        let msg = DaemonMessage::Reconnected;
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "reconnected");
    }

    /// Reconnected 메시지 역직렬화 검증
    #[test]
    fn test_daemon_message_reconnected_deserialization() {
        let json = r#"{"type":"reconnected"}"#;
        let msg: DaemonMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, DaemonMessage::Reconnected));
    }

    /// Disconnected → Reconnected 시퀀스가 newline-delimited 프로토콜로 전송되는지 검증
    #[test]
    fn test_disconnected_reconnected_sequence_protocol() {
        let messages = vec![
            DaemonMessage::Disconnected {
                reason: "daemon killed".to_string(),
            },
            DaemonMessage::Reconnected,
        ];

        let mut wire = String::new();
        for msg in &messages {
            wire.push_str(&serde_json::to_string(msg).unwrap());
            wire.push('\n');
        }

        let parsed: Vec<DaemonMessage> = wire
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();

        assert_eq!(parsed.len(), 2);
        match &parsed[0] {
            DaemonMessage::Disconnected { reason } => {
                assert_eq!(reason, "daemon killed");
            }
            _ => panic!("Expected Disconnected"),
        }
        assert!(matches!(parsed[1], DaemonMessage::Reconnected));
    }

    #[test]
    fn test_health_check_command_serialize() {
        let cmd = ClientCommand::HealthCheck;
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("health_check"));
        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, ClientCommand::HealthCheck));
    }

    #[test]
    fn test_health_check_result_message_serialize() {
        let msg = DaemonMessage::HealthCheckResult {
            uptime_secs: 3600,
            active_connections: 5,
            total_transactions: 1234,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("health_check_result"));

        let deserialized: DaemonMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            DaemonMessage::HealthCheckResult {
                uptime_secs,
                active_connections,
                total_transactions,
            } => {
                assert_eq!(uptime_secs, 3600);
                assert_eq!(active_connections, 5);
                assert_eq!(total_transactions, 1234);
            }
            _ => panic!("Expected HealthCheckResult"),
        }
    }

    #[test]
    fn test_health_check_result_roundtrip() {
        let json = r#"{"type":"health_check_result","uptime_secs":120,"active_connections":0,"total_transactions":42}"#;
        let msg: DaemonMessage = serde_json::from_str(json).unwrap();
        match msg {
            DaemonMessage::HealthCheckResult {
                uptime_secs,
                active_connections,
                total_transactions,
            } => {
                assert_eq!(uptime_secs, 120);
                assert_eq!(active_connections, 0);
                assert_eq!(total_transactions, 42);
            }
            _ => panic!("Expected HealthCheckResult"),
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

        // 역직렬화 검증
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

impl std::fmt::Display for InterceptRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.enabled { "활성" } else { "비활성" };
        let action_desc = match &self.action {
            InterceptAction::Block { status_code, .. } => {
                format!("Block({})", status_code)
            }
            InterceptAction::ModifyRequest { .. } => "ModifyRequest".to_string(),
            InterceptAction::ModifyResponse { set_status, .. } => {
                if let Some(status) = set_status {
                    format!("ModifyResponse(status={})", status)
                } else {
                    "ModifyResponse".to_string()
                }
            }
            InterceptAction::MapLocal {
                file_path,
                status_code,
                ..
            } => {
                format!("MapLocal({}, status={})", file_path, status_code)
            }
            InterceptAction::MapRemote {
                target_url,
                preserve_path,
            } => {
                format!("MapRemote({}, preserve_path={})", target_url, preserve_path)
            }
            InterceptAction::Rewrite {
                target,
                match_pattern,
                ..
            } => {
                format!("Rewrite({:?}, pattern={})", target, match_pattern)
            }
        };
        let method_str = self.method.as_deref().unwrap_or("*");
        write!(
            f,
            "[{}] {} ({} {}) -> {} [{}]",
            self.id, self.name, method_str, self.pattern, action_desc, status
        )
    }
}
