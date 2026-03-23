use std::sync::Arc;

use cheolsu_ops::context::OpsContext;
use proxy_daemon::DaemonConnection;
use rmcp::{handler::server::router::tool::ToolRouter, model::*, tool_handler, ServerHandler};
use tokio::sync::Mutex as TokioMutex;

use crate::store::Store;

#[derive(Clone)]
pub(crate) struct CheolsuMcpServer {
    pub(crate) store: Store,
    pub(crate) daemon_conn: Arc<TokioMutex<Option<DaemonConnection>>>,
    tool_router: ToolRouter<Self>,
}

impl CheolsuMcpServer {
    pub(crate) fn new(store: Store, conn: Option<DaemonConnection>) -> Self {
        Self {
            store,
            daemon_conn: Arc::new(TokioMutex::new(conn)),
            tool_router: Self::tool_router(),
        }
    }

    pub(crate) fn ops_ctx(&self) -> OpsContext {
        OpsContext {
            store: self.store.clone(),
            daemon_conn: self.daemon_conn.clone(),
        }
    }

    fn tool_router() -> ToolRouter<Self> {
        ToolRouter::<Self>::new()
            .with_route((Self::search_traffic_tool_attr(), Self::search_traffic))
            .with_route((Self::get_transaction_tool_attr(), Self::get_transaction))
            .with_route((
                Self::get_websocket_messages_tool_attr(),
                Self::get_websocket_messages,
            ))
            .with_route((Self::replay_request_tool_attr(), Self::replay_request))
            .with_route((Self::list_rules_tool_attr(), Self::list_rules))
            .with_route((Self::add_rule_tool_attr(), Self::add_rule))
            .with_route((Self::remove_rule_tool_attr(), Self::remove_rule))
            .with_route((Self::load_script_tool_attr(), Self::load_script))
            .with_route((Self::unload_script_tool_attr(), Self::unload_script))
            .with_route((Self::diff_transactions_tool_attr(), Self::diff_transactions))
            .with_route((Self::proxy_status_tool_attr(), Self::proxy_status))
            .with_route((Self::clear_traffic_tool_attr(), Self::clear_traffic))
            .with_route((Self::list_breakpoints_tool_attr(), Self::list_breakpoints))
            .with_route((
                Self::list_host_mappings_tool_attr(),
                Self::list_host_mappings,
            ))
            .with_route((Self::add_breakpoint_tool_attr(), Self::add_breakpoint))
            .with_route((Self::remove_breakpoint_tool_attr(), Self::remove_breakpoint))
            .with_route((Self::add_host_mapping_tool_attr(), Self::add_host_mapping))
            .with_route((
                Self::remove_host_mapping_tool_attr(),
                Self::remove_host_mapping,
            ))
            .with_route((
                Self::list_pending_breakpoints_tool_attr(),
                Self::list_pending_breakpoints,
            ))
            .with_route((
                Self::resolve_breakpoint_tool_attr(),
                Self::resolve_breakpoint,
            ))
            .with_route((Self::save_session_tool_attr(), Self::save_session))
            .with_route((Self::load_session_tool_attr(), Self::load_session))
            .with_route((
                Self::generate_openapi_spec_tool_attr(),
                Self::generate_openapi_spec,
            ))
            // SSE
            .with_route((Self::get_sse_events_tool_attr(), Self::get_sse_events))
            // Server Replay
            .with_route((
                Self::list_server_replay_tool_attr(),
                Self::list_server_replay,
            ))
            .with_route((Self::add_server_replay_tool_attr(), Self::add_server_replay))
            .with_route((
                Self::remove_server_replay_tool_attr(),
                Self::remove_server_replay,
            ))
            .with_route((
                Self::clear_server_replay_tool_attr(),
                Self::clear_server_replay,
            ))
            // Advanced Replay
            .with_route((Self::replay_sequence_tool_attr(), Self::replay_sequence))
            .with_route((Self::advanced_repeat_tool_attr(), Self::advanced_repeat))
            // Settings
            .with_route((
                Self::update_upstream_proxy_tool_attr(),
                Self::update_upstream_proxy,
            ))
            .with_route((Self::update_throttle_tool_attr(), Self::update_throttle))
            .with_route((Self::update_proxy_auth_tool_attr(), Self::update_proxy_auth))
            .with_route((
                Self::update_connection_strategy_tool_attr(),
                Self::update_connection_strategy,
            ))
            .with_route((
                Self::update_quick_settings_tool_attr(),
                Self::update_quick_settings,
            ))
            .with_route((
                Self::update_client_certificate_tool_attr(),
                Self::update_client_certificate,
            ))
            .with_route((
                Self::update_ssl_proxying_list_tool_attr(),
                Self::update_ssl_proxying_list,
            ))
            // Reverse Proxy
            .with_route((
                Self::list_reverse_proxy_rules_tool_attr(),
                Self::list_reverse_proxy_rules,
            ))
            .with_route((
                Self::add_reverse_proxy_rule_tool_attr(),
                Self::add_reverse_proxy_rule,
            ))
            .with_route((
                Self::remove_reverse_proxy_rule_tool_attr(),
                Self::remove_reverse_proxy_rule,
            ))
            // Export
            .with_route((Self::export_har_tool_attr(), Self::export_har))
            // Analytics
            .with_route((
                Self::analyze_performance_tool_attr(),
                Self::analyze_performance,
            ))
            .with_route((Self::analyze_errors_tool_attr(), Self::analyze_errors))
            .with_route((Self::analyze_endpoints_tool_attr(), Self::analyze_endpoints))
            .with_route((Self::detect_duplicates_tool_attr(), Self::detect_duplicates))
            .with_route((Self::detect_n_plus_one_tool_attr(), Self::detect_n_plus_one))
            .with_route((
                Self::analyze_traffic_timeline_tool_attr(),
                Self::analyze_traffic_timeline,
            ))
            .with_route((Self::analyze_full_tool_attr(), Self::analyze_full))
    }
}

#[tool_handler]
impl ServerHandler for CheolsuMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "Cheolsu Proxy MCP Server — search/inspect captured HTTP & WebSocket traffic, replay requests, and manage intercept rules. Start the Cheolsu Proxy app first.".to_string(),
            )
    }
}
