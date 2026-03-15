use proxy_daemon::ReverseProxyRule;
use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{add_and_sync, list_items, next_reverse_proxy_id, remove_and_sync};
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "List all reverse proxy rules. Reverse proxy rules route incoming requests with relative URIs (no scheme/authority) to backend servers based on the Host header pattern."
    )]
    pub(crate) async fn list_reverse_proxy_rules(&self) -> Result<CallToolResult, McpError> {
        list_items(
            &self.store.reverse_proxy_rules,
            "reverse proxy rules",
            "No reverse proxy rules configured.",
        )
    }

    #[tool(
        description = "Add a reverse proxy rule. Routes requests matching the Host header pattern to the specified backend server. Supports wildcard patterns (e.g., \"*.local\"). When a request with a relative URI arrives, the proxy matches the Host header against configured rules and rewrites the URI to target the backend."
    )]
    pub(crate) async fn add_reverse_proxy_rule(
        &self,
        Parameters(p): Parameters<AddReverseProxyRuleParams>,
    ) -> Result<CallToolResult, McpError> {
        let backend_scheme = p.backend_scheme.unwrap_or_else(|| "http".to_string());
        if backend_scheme != "http" && backend_scheme != "https" {
            return crate::helpers::tool_error(format!(
                "Invalid backend_scheme '{}'. Must be 'http' or 'https'.",
                backend_scheme
            ));
        }

        let id = next_reverse_proxy_id();
        let rule = ReverseProxyRule {
            id: id.clone(),
            match_host: p.match_host,
            backend_scheme,
            backend_host: p.backend_host,
            backend_port: p.backend_port,
            rewrite_host: p.rewrite_host.unwrap_or(true),
            enabled: true,
        };

        add_and_sync(
            &self.store.reverse_proxy_rules,
            rule,
            &id,
            "Reverse proxy rule",
            || self.send_reverse_proxy_rules(),
        )
        .await
    }

    #[tool(description = "Remove a reverse proxy rule by its ID.")]
    pub(crate) async fn remove_reverse_proxy_rule(
        &self,
        Parameters(p): Parameters<RemoveReverseProxyRuleParams>,
    ) -> Result<CallToolResult, McpError> {
        remove_and_sync(
            &self.store.reverse_proxy_rules,
            &p.id,
            |r| &r.id,
            "Reverse proxy rule",
            || self.send_reverse_proxy_rules(),
        )
        .await
    }
}
