pub mod client;
pub mod daemon;
pub mod handler;
pub mod protocol;
pub mod system_proxy;

// Re-exports for convenience
pub use client::{connect_to_daemon, ensure_daemon, is_daemon_running, DaemonConnection};
pub use daemon::{check_and_cleanup_stale_lock, lock_file_path, run_daemon, uds_socket_path};
pub use handler::{create_hybrid_client, LoggingHandler, WsEvent};
pub use protocol::{ClientCommand, DaemonMessage, InterceptAction, InterceptRule, ProxyLockInfo};
pub use system_proxy::{get_proxy_status, set_proxy, ProxyStatus};

// Re-export cache utilities from proxyapi_v2
pub use proxyapi_v2::certificate_authority::{clean_all_cache, clean_old_cache};
