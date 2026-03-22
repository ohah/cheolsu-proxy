use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::op_result_to_mcp;
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "Configure upstream proxy. All proxy traffic will be forwarded through this proxy. Set enabled=false to disable."
    )]
    pub(crate) async fn update_upstream_proxy(
        &self,
        Parameters(p): Parameters<UpdateUpstreamProxyParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::settings::update_upstream_proxy(&self.ops_ctx(), p).await)
    }

    #[tool(
        description = "Configure network throttling to simulate slow connections. Set enabled=false to disable. Rates are in bytes/sec."
    )]
    pub(crate) async fn update_throttle(
        &self,
        Parameters(p): Parameters<UpdateThrottleParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::settings::update_throttle(&self.ops_ctx(), p).await)
    }

    #[tool(
        description = "Configure proxy authentication. When enabled, clients must provide credentials to use the proxy. Supports basic, bearer, and apikey methods."
    )]
    pub(crate) async fn update_proxy_auth(
        &self,
        Parameters(p): Parameters<UpdateProxyAuthParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::settings::update_proxy_auth(&self.ops_ctx(), p).await)
    }

    #[tool(
        description = "Set the connection strategy for upstream connections. Options: 'lazy' (connect on first request), 'eager' (pre-connect), 'eager_with_fallback' (pre-connect with lazy fallback)."
    )]
    pub(crate) async fn update_connection_strategy(
        &self,
        Parameters(p): Parameters<UpdateConnectionStrategyParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(
            cheolsu_ops::settings::update_connection_strategy(&self.ops_ctx(), p).await,
        )
    }

    #[tool(
        description = "Update quick proxy settings. no_caching removes cache headers, block_cookies removes cookie headers, no_gzip disables gzip encoding, block_quic strips Alt-Svc headers to prevent QUIC/HTTP3 upgrades."
    )]
    pub(crate) async fn update_quick_settings(
        &self,
        Parameters(p): Parameters<UpdateQuickSettingsParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::settings::update_quick_settings(&self.ops_ctx(), p).await)
    }

    #[tool(
        description = "Configure client certificate for mTLS (mutual TLS). The proxy will present this certificate when connecting to upstream servers that require client authentication."
    )]
    pub(crate) async fn update_client_certificate(
        &self,
        Parameters(p): Parameters<UpdateClientCertificateParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::settings::update_client_certificate(&self.ops_ctx(), p).await)
    }

    #[tool(
        description = "Configure SSL proxying list. In blacklist mode (default), all domains are intercepted except listed ones. In whitelist mode, only listed domains are intercepted."
    )]
    pub(crate) async fn update_ssl_proxying_list(
        &self,
        Parameters(p): Parameters<UpdateSslProxyingListParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::settings::update_ssl_proxying_list(&self.ops_ctx(), p).await)
    }
}
