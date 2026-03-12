use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::breakpoint::BreakpointManager;
use crate::protocol::{HostMapping, InterceptRule, ServerReplayEntry};

use super::{SseState, WebSocketState};

/// 요청별 임시 상태 (매 요청마다 초기화)
#[derive(Clone)]
pub(crate) struct RequestState {
    pub(crate) req: Option<proxy_v2_models::ProxiedRequest>,
    pub(crate) res: Option<proxy_v2_models::ProxiedResponse>,
}

/// 빠른 설정 (No Caching, Block Cookies, No Gzip)
#[derive(Clone, Copy, Debug, Default)]
pub struct QuickSettings {
    pub no_caching: bool,
    pub block_cookies: bool,
    pub no_gzip: bool,
}

/// 프록시 전역 설정 (세션 수명 동안 유지)
#[derive(Clone)]
pub(crate) struct ProxyConfig {
    pub(crate) cache_dir: Option<std::path::PathBuf>,
    /// CA 인증서 DER 바이트 (외부 기기 인증서 다운로드용, zero-copy)
    pub(crate) ca_cert_der: Option<Bytes>,
    /// 빠른 설정 (No Caching, Block Cookies)
    pub(crate) quick_settings: Arc<tokio::sync::RwLock<QuickSettings>>,
    /// 프록시 인증 설정
    pub(crate) proxy_auth: Arc<tokio::sync::RwLock<Option<crate::protocol::ProxyAuthConfig>>>,
    /// 요청 바디 최대 크기 (None이면 제한 없음)
    pub(crate) max_body_size: Option<usize>,
}

/// 인터셉트 규칙 및 스크립트 엔진
#[derive(Clone)]
pub(crate) struct InterceptEngine {
    pub(crate) intercept_rules: Arc<RwLock<Vec<InterceptRule>>>,
    pub(crate) server_replay_entries: Arc<RwLock<Vec<ServerReplayEntry>>>,
    pub(crate) host_mappings: Arc<RwLock<Vec<HostMapping>>>,
    pub(crate) script_handle: scripting::ScriptHandle,
    /// SSL Proxying 모드
    pub(crate) ssl_proxying_mode: Arc<RwLock<crate::protocol::SslProxyingMode>>,
    /// SSL Proxying 엔트리 목록
    pub(crate) ssl_proxying_entries: Arc<RwLock<Vec<crate::protocol::SslProxyingEntry>>>,
}

/// HTTP 및 WebSocket 요청/응답을 로깅하는 핸들러
#[derive(Clone)]
pub struct LoggingHandler {
    pub(crate) sender: tokio::sync::mpsc::Sender<proxy_v2_models::RequestInfo>,
    pub(crate) request: RequestState,
    pub(crate) config: ProxyConfig,
    pub(crate) intercept: InterceptEngine,
    pub(crate) ws: WebSocketState,
    pub(crate) sse: SseState,
    pub(crate) breakpoint_manager: Option<BreakpointManager>,
}

impl LoggingHandler {
    pub fn new(
        sender: tokio::sync::mpsc::Sender<proxy_v2_models::RequestInfo>,
        cache_dir: std::path::PathBuf,
    ) -> Self {
        Self {
            sender,
            request: RequestState {
                req: None,
                res: None,
            },
            config: ProxyConfig {
                cache_dir: Some(cache_dir),
                ca_cert_der: None,
                quick_settings: Arc::new(tokio::sync::RwLock::new(QuickSettings::default())),
                proxy_auth: Arc::new(tokio::sync::RwLock::new(None)),
                max_body_size: None,
            },
            intercept: InterceptEngine {
                intercept_rules: Arc::new(RwLock::new(Vec::new())),
                server_replay_entries: Arc::new(RwLock::new(Vec::new())),
                host_mappings: Arc::new(RwLock::new(Vec::new())),
                script_handle: scripting::ScriptHandle::new(),
                ssl_proxying_mode: Arc::new(RwLock::new(
                    crate::protocol::SslProxyingMode::default(),
                )),
                ssl_proxying_entries: Arc::new(RwLock::new(Vec::new())),
            },
            ws: WebSocketState {
                ws_sender: None,
                ws_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                mqtt_versions: Arc::new(parking_lot::Mutex::new(std::collections::HashMap::new())),
            },
            sse: SseState {
                sse_sender: None,
                sse_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            },
            breakpoint_manager: None,
        }
    }

    /// CA 인증서 DER 바이트를 설정합니다 (외부 기기 인증서 다운로드용)
    pub fn with_ca_cert_der(mut self, der: Vec<u8>) -> Self {
        self.config.ca_cert_der = Some(Bytes::from(der));
        self
    }

    pub fn with_ws_sender(mut self, ws_sender: tokio::sync::mpsc::Sender<super::WsEvent>) -> Self {
        self.ws.ws_sender = Some(ws_sender);
        self
    }

    pub fn with_sse_sender(
        mut self,
        sse_sender: tokio::sync::mpsc::Sender<super::SseEvent>,
    ) -> Self {
        self.sse.sse_sender = Some(sse_sender);
        self
    }

    pub fn with_script_handle(mut self, handle: scripting::ScriptHandle) -> Self {
        self.intercept.script_handle = handle;
        self
    }

    pub fn with_breakpoint_manager(mut self, mgr: BreakpointManager) -> Self {
        self.breakpoint_manager = Some(mgr);
        self
    }

    pub fn with_quick_settings(
        mut self,
        quick_settings: Arc<tokio::sync::RwLock<QuickSettings>>,
    ) -> Self {
        self.config.quick_settings = quick_settings;
        self
    }

    pub fn breakpoint_manager(&self) -> Option<&BreakpointManager> {
        self.breakpoint_manager.as_ref()
    }

    /// 인터셉트 규칙 업데이트
    pub async fn update_intercept_rules(&self, rules: Vec<InterceptRule>) {
        let mut rules_guard = self.intercept.intercept_rules.write().await;
        tracing::info!("[Intercept] 규칙 업데이트: {} 개", rules.len());
        *rules_guard = rules;
    }

    /// 서버 리플레이 엔트리 업데이트
    pub async fn update_server_replay_entries(&self, entries: Vec<ServerReplayEntry>) {
        let mut entries_guard = self.intercept.server_replay_entries.write().await;
        tracing::info!("[ServerReplay] 엔트리 업데이트: {} 개", entries.len());
        *entries_guard = entries;
    }

    /// Update host mappings
    pub async fn update_host_mappings(&self, mappings: Vec<HostMapping>) {
        let mut mappings_guard = self.intercept.host_mappings.write().await;
        tracing::info!("[HostMapping] mappings updated: {} entries", mappings.len());
        *mappings_guard = mappings;
    }

    /// SSL Proxying 모드 및 목록 업데이트
    pub async fn update_ssl_proxying(
        &self,
        mode: crate::protocol::SslProxyingMode,
        entries: Vec<crate::protocol::SslProxyingEntry>,
    ) {
        // 로그를 먼저 출력하여 mode.clone() 제거
        tracing::info!(
            "[SSLProxying] 업데이트: mode={:?}, {} 개",
            mode,
            entries.len()
        );
        let mut mode_guard = self.intercept.ssl_proxying_mode.write().await;
        *mode_guard = mode;
        drop(mode_guard);
        let mut entries_guard = self.intercept.ssl_proxying_entries.write().await;
        *entries_guard = entries;
    }

    /// 스크립트 핸들 반환
    pub fn script_handle(&self) -> &scripting::ScriptHandle {
        &self.intercept.script_handle
    }

    pub fn with_proxy_auth(
        mut self,
        proxy_auth: Arc<tokio::sync::RwLock<Option<crate::protocol::ProxyAuthConfig>>>,
    ) -> Self {
        self.config.proxy_auth = proxy_auth;
        self
    }

    /// 요청 바디 최대 크기를 설정합니다 (None이면 제한 없음)
    pub fn with_max_body_size(mut self, max_body_size: Option<usize>) -> Self {
        self.config.max_body_size = max_body_size;
        self
    }
}
