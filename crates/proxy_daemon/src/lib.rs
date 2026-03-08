pub mod client;
pub mod client_handler;
pub mod curl_fallback;
pub mod daemon;
pub mod error;
pub mod handler;
pub mod intercept;
pub mod protocol;
pub mod proxy_runner;
pub mod script_bridge;
pub mod system_proxy;
pub mod tls_client;

// Re-exports for convenience
pub use client::{connect_to_daemon, ensure_daemon, is_daemon_running, DaemonConnection};
pub use daemon::{check_and_cleanup_stale_lock, lock_file_path, run_daemon, uds_socket_path};
pub use error::DaemonError;
pub use handler::{create_hybrid_client, LoggingHandler, WsEvent};
pub use protocol::{
    ClientCommand, DaemonMessage, InterceptAction, InterceptRule, ProxyLockInfo, ServerReplayEntry,
};
pub use proxyapi_v2::upstream_proxy::{UpstreamProxyAuth, UpstreamProxyConfig};
pub use system_proxy::{get_proxy_status, set_proxy, ProxyStatus};

// Re-export cache utilities from proxyapi_v2
pub use proxyapi_v2::certificate_authority::{clean_all_cache, clean_old_cache};
