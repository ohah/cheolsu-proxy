use super::auth::ProxyAuthConfig;
use super::breakpoint_types::{BreakpointAction, BreakpointData, BreakpointPhase, BreakpointRule};
use super::host_mapping::HostMapping;
use super::intercept::{InterceptRule, ServerReplayEntry};
use super::ssl::{
    ClientCertConfig, RequestClientCertConfig, SslProxyingEntry, SslProxyingMode,
    TlsPassthroughEntry,
};
use proxy_v2_models::{
    RequestInfo, SseConnectionEvent, SseEventInfo, WsConnectionEvent, WsMessageInfo,
};
use proxyapi_v2::throttle::ThrottleConfig;
use proxyapi_v2::upstream_proxy::UpstreamProxyConfig;
use serde::{Deserialize, Serialize};

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
    #[serde(rename = "sse_event")]
    SseEvent { data: SseEventInfo },
    #[serde(rename = "sse_connection")]
    SseConnection { data: SseConnectionEvent },
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
    /// SSL Proxying 목록 업데이트됨
    #[serde(rename = "ssl_proxying_list_updated")]
    SslProxyingListUpdated {
        #[serde(default)]
        mode: SslProxyingMode,
        entries: Vec<SslProxyingEntry>,
    },
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
    /// TLS Passthrough 바이패스 목록 업데이트됨
    #[serde(rename = "tls_passthrough_updated")]
    TlsPassthroughUpdated { entries: Vec<TlsPassthroughEntry> },
    /// 데몬 연결이 끊어졌음을 알리는 메시지
    #[serde(rename = "disconnected")]
    Disconnected { reason: String },
    /// 데몬에 재연결되었음을 알리는 메시지
    #[serde(rename = "reconnected")]
    Reconnected,
    /// 클라이언트 인증서 요청 설정 업데이트됨
    #[serde(rename = "request_client_cert_updated")]
    RequestClientCertUpdated {
        config: Option<RequestClientCertConfig>,
    },
    /// 메트릭 조회 결과
    #[serde(rename = "metrics_result")]
    MetricsResult {
        active_requests: i64,
        total_requests: u64,
        total_bytes_sent: u64,
        total_bytes_received: u64,
        total_tls_handshakes: u64,
        total_tls_failures: u64,
        total_connection_failures: u64,
        total_timeouts: u64,
        uptime_secs: u64,
    },
    /// 도메인별 통계 조회 결과
    #[serde(rename = "domain_stats_result")]
    DomainStatsResult {
        stats: Vec<crate::metrics_aggregator::DomainStatsEntry>,
    },
    /// 최근 에러 조회 결과
    #[serde(rename = "recent_errors_result")]
    RecentErrorsResult {
        errors: Vec<crate::metrics_aggregator::ErrorEntry>,
    },
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
    /// SSL Proxying 목록 업데이트
    #[serde(rename = "update_ssl_proxying_list")]
    UpdateSslProxyingList {
        #[serde(default)]
        mode: SslProxyingMode,
        entries: Vec<SslProxyingEntry>,
    },
    /// 프록시 인증 설정 업데이트
    #[serde(rename = "update_proxy_auth")]
    UpdateProxyAuth { config: ProxyAuthConfig },
    /// 클라이언트 인증서 설정 업데이트 (mTLS)
    #[serde(rename = "update_client_certificate")]
    UpdateClientCertificate { config: Option<ClientCertConfig> },
    /// TLS Passthrough 목록 조회
    #[serde(rename = "get_tls_passthrough_list")]
    GetTlsPassthroughList,
    /// TLS Passthrough 특정 도메인 바이패스 해제
    #[serde(rename = "remove_tls_passthrough")]
    RemoveTlsPassthrough { host: String },
    /// TLS Passthrough 전체 초기화
    #[serde(rename = "clear_tls_passthrough")]
    ClearTlsPassthrough,
    /// 헬스체크 요청
    #[serde(rename = "health_check")]
    HealthCheck,
    /// 클라이언트 인증서 요청 설정 업데이트 (프록시 → 클라이언트)
    #[serde(rename = "update_request_client_cert")]
    UpdateRequestClientCert {
        config: Option<RequestClientCertConfig>,
    },
    /// 연결 전략 업데이트 (Lazy, Eager, EagerWithFallback)
    #[serde(rename = "update_connection_strategy")]
    UpdateConnectionStrategy { strategy: String },
    /// 메트릭 조회
    #[serde(rename = "get_metrics")]
    GetMetrics,
    /// 도메인별 통계 조회
    #[serde(rename = "get_domain_stats")]
    GetDomainStats {
        #[serde(default)]
        domain: Option<String>,
    },
    /// 최근 에러 조회
    #[serde(rename = "get_recent_errors")]
    GetRecentErrors {
        #[serde(default)]
        limit: Option<usize>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProxyLockInfo {
    pub pid: u32,
    pub port: u16,
    pub uds_path: String,
}
