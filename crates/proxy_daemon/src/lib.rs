pub(crate) mod breakpoint;
pub(crate) mod cert_distribution;
pub mod client;
pub mod client_handler;
pub mod curl_fallback;
pub mod daemon;
pub(crate) mod diff;
pub mod error;
pub mod handler;
pub(crate) mod host_mapping;
pub mod intercept;
pub mod metrics_aggregator;
pub mod net_utils;
pub(crate) mod pattern_utils;
pub mod protocol;
pub mod proxy_runner;
pub mod script_bridge;
pub(crate) mod session;
pub(crate) mod ssl_proxying;
pub mod system_proxy;
pub mod tls_client;

// Re-exports for convenience
pub use breakpoint::BreakpointManager;
pub use client::{
    connect_to_daemon, ensure_daemon, is_daemon_running, CommandSender, DaemonConnection,
};
pub use daemon::{check_and_cleanup_stale_lock, lock_file_path, run_daemon, uds_socket_path};
pub use diff::{
    diff_headers, diff_json, diff_text, format_diff_text, is_text_data_type, BodyDiff, DiffLine,
    HeaderDiff, JsonDiffEntry, TrafficDiff, TransactionPartDiff,
};
pub use error::DaemonError;
pub use handler::{
    create_hybrid_client, create_hybrid_client_with_cert, validate_client_cert_config,
    LoggingHandler, QuickSettings, WsEvent,
};
pub use net_utils::get_local_ips;
pub use protocol::{
    BreakpointAction, BreakpointData, BreakpointPhase, BreakpointRule, CertificateInfo,
    ClientCertConfig, ClientCommand, DaemonMessage, DomainClientCertConfig, HostMapping,
    InterceptAction, InterceptRule, ProxyAuthConfig, ProxyLockInfo, RequestClientCertConfig,
    ServerReplayEntry, SslProxyingEntry, SslProxyingMode, TlsPassthroughEntry,
};
pub use proxy_v2_models::RequestInfo;
pub use proxyapi_v2::certificate_authority::{clean_all_cache, clean_old_cache};
pub use proxyapi_v2::throttle::{ThrottleConfig, ThrottlePreset};
pub use proxyapi_v2::upstream_proxy::{UpstreamProxyAuth, UpstreamProxyConfig};
pub use session::{
    ensure_extension, import_har, import_har_file, SessionFile, SessionMetadata, SessionTransaction,
};
pub use system_proxy::{get_proxy_status, set_proxy, ProxyStatus};
pub use tls_client::{
    get_custom_ca_info, parse_certificate_info, parse_certificate_info_from_bytes, parse_pkcs12,
    remove_custom_ca, save_custom_ca, validate_ca_certificate, validate_ca_certificate_from_bytes,
    validate_cert_key_pair,
};
