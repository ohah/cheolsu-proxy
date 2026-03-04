pub mod client;
pub mod daemon;
pub mod handler;
pub mod protocol;
pub mod system_proxy;

// Re-exports for convenience
pub use client::{connect_to_daemon, ensure_daemon, is_daemon_running, DaemonConnection};
pub use daemon::{check_and_cleanup_stale_lock, lock_file_path, run_daemon, uds_socket_path};
pub use handler::{create_hybrid_client, LoggingHandler};
pub use protocol::{ClientCommand, DaemonMessage, ProxyLockInfo};
pub use system_proxy::{get_proxy_status, set_proxy, ProxyStatus};
