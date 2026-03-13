use std::sync::Arc;

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
    }

    pub(crate) async fn send_rules(&self) -> Result<(), String> {
        let cmd = {
            let rules = self.store.rules.lock();
            proxy_daemon::ClientCommand::UpdateInterceptRules {
                rules: rules.clone(),
            }
        };
        crate::helpers::with_daemon_conn(&self.daemon_conn, &cmd).await
    }

    pub(crate) async fn send_host_mappings(&self) -> Result<(), String> {
        let cmd = {
            let mappings = self.store.host_mappings.lock();
            proxy_daemon::ClientCommand::UpdateHostMappings {
                mappings: mappings.clone(),
            }
        };
        crate::helpers::with_daemon_conn(&self.daemon_conn, &cmd).await
    }

    pub(crate) async fn send_breakpoint_rules(&self) -> Result<(), String> {
        let cmd = {
            let rules = self.store.breakpoint_rules.lock();
            proxy_daemon::ClientCommand::UpdateBreakpointRules {
                rules: rules.clone(),
            }
        };
        crate::helpers::with_daemon_conn(&self.daemon_conn, &cmd).await
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
