use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::op_result_to_mcp;
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "List all reverse proxy rules. Reverse proxy rules route incoming requests with relative URIs (no scheme/authority) to backend servers based on the Host header pattern."
    )]
    pub(crate) async fn list_reverse_proxy_rules(&self) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::reverse_proxy::list_reverse_proxy_rules(
            &self.ops_ctx(),
        ))
    }

    #[tool(
        description = "Add a reverse proxy rule. Routes requests matching the Host header pattern to the specified backend server. Supports wildcard patterns (e.g., \"*.local\"). When a request with a relative URI arrives, the proxy matches the Host header against configured rules and rewrites the URI to target the backend."
    )]
    pub(crate) async fn add_reverse_proxy_rule(
        &self,
        Parameters(p): Parameters<AddReverseProxyRuleParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(
            cheolsu_ops::reverse_proxy::add_reverse_proxy_rule(&self.ops_ctx(), p).await,
        )
    }

    #[tool(description = "Remove a reverse proxy rule by its ID.")]
    pub(crate) async fn remove_reverse_proxy_rule(
        &self,
        Parameters(p): Parameters<RemoveReverseProxyRuleParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(
            cheolsu_ops::reverse_proxy::remove_reverse_proxy_rule(&self.ops_ctx(), p).await,
        )
    }
}
