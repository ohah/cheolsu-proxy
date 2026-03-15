use proxy_daemon::ClientCommand;
use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{tool_error, tool_ok, with_daemon_conn};
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
        let config = if p.enabled {
            let Some(host) = p.host else {
                return tool_error("host is required when enabling upstream proxy.");
            };
            Some(proxy_daemon::UpstreamProxyConfig {
                host,
                port: p.port.unwrap_or(8080),
                auth: match (p.username, p.password) {
                    (Some(u), Some(pw)) => Some(proxy_daemon::UpstreamProxyAuth {
                        username: u,
                        password: pw,
                    }),
                    _ => None,
                },
                bypass: p.bypass.unwrap_or_default(),
            })
        } else {
            None
        };
        let cmd = ClientCommand::UpdateUpstreamProxy { config };
        match with_daemon_conn(&self.daemon_conn, &cmd).await {
            Ok(()) => {
                if p.enabled {
                    tool_ok("Upstream proxy enabled.")
                } else {
                    tool_ok("Upstream proxy disabled.")
                }
            }
            Err(e) => tool_error(format!("Failed to update upstream proxy: {}", e)),
        }
    }

    #[tool(
        description = "Configure network throttling to simulate slow connections. Set enabled=false to disable. Rates are in bytes/sec."
    )]
    pub(crate) async fn update_throttle(
        &self,
        Parameters(p): Parameters<UpdateThrottleParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = if p.enabled {
            Some(proxy_daemon::ThrottleConfig {
                enabled: true,
                download_rate: p.download_rate,
                upload_rate: p.upload_rate,
                latency_ms: p.latency_ms.unwrap_or(0),
            })
        } else {
            None
        };
        let cmd = ClientCommand::UpdateThrottle { config };
        match with_daemon_conn(&self.daemon_conn, &cmd).await {
            Ok(()) => {
                if p.enabled {
                    let mut parts = Vec::new();
                    if let Some(dl) = p.download_rate {
                        parts.push(format!("download: {} B/s", dl));
                    }
                    if let Some(ul) = p.upload_rate {
                        parts.push(format!("upload: {} B/s", ul));
                    }
                    if let Some(lat) = p.latency_ms {
                        if lat > 0 {
                            parts.push(format!("latency: {}ms", lat));
                        }
                    }
                    tool_ok(format!("Throttle enabled ({})", parts.join(", ")))
                } else {
                    tool_ok("Throttle disabled.")
                }
            }
            Err(e) => tool_error(format!("Failed to update throttle: {}", e)),
        }
    }

    #[tool(
        description = "Configure proxy authentication. When enabled, clients must provide credentials to use the proxy. Supports basic, bearer, and apikey methods."
    )]
    pub(crate) async fn update_proxy_auth(
        &self,
        Parameters(p): Parameters<UpdateProxyAuthParams>,
    ) -> Result<CallToolResult, McpError> {
        let method = match p.method.as_deref() {
            Some("bearer") => proxy_daemon::AuthMethod::Bearer,
            Some("apikey") => proxy_daemon::AuthMethod::ApiKey,
            _ => proxy_daemon::AuthMethod::Basic,
        };
        let config = proxy_daemon::ProxyAuthConfig {
            enabled: p.enabled,
            method,
            username: p.username.unwrap_or_default(),
            password: p.password.unwrap_or_default(),
            token: p.token,
        };
        let cmd = ClientCommand::UpdateProxyAuth { config };
        match with_daemon_conn(&self.daemon_conn, &cmd).await {
            Ok(()) => {
                if p.enabled {
                    tool_ok("Proxy authentication enabled.")
                } else {
                    tool_ok("Proxy authentication disabled.")
                }
            }
            Err(e) => tool_error(format!("Failed to update proxy auth: {}", e)),
        }
    }

    #[tool(
        description = "Set the connection strategy for upstream connections. Options: 'lazy' (connect on first request), 'eager' (pre-connect), 'eager_with_fallback' (pre-connect with lazy fallback)."
    )]
    pub(crate) async fn update_connection_strategy(
        &self,
        Parameters(p): Parameters<UpdateConnectionStrategyParams>,
    ) -> Result<CallToolResult, McpError> {
        let strategy = p.strategy.to_lowercase();
        if !["lazy", "eager", "eager_with_fallback"].contains(&strategy.as_str()) {
            return tool_error("Invalid strategy. Use: 'lazy', 'eager', or 'eager_with_fallback'.");
        }
        let cmd = ClientCommand::UpdateConnectionStrategy {
            strategy: strategy.clone(),
        };
        match with_daemon_conn(&self.daemon_conn, &cmd).await {
            Ok(()) => tool_ok(format!("Connection strategy set to '{}'.", strategy)),
            Err(e) => tool_error(format!("Failed to update connection strategy: {}", e)),
        }
    }

    #[tool(
        description = "Update quick proxy settings. no_caching removes cache headers, block_cookies removes cookie headers, no_gzip disables gzip encoding."
    )]
    pub(crate) async fn update_quick_settings(
        &self,
        Parameters(p): Parameters<UpdateQuickSettingsParams>,
    ) -> Result<CallToolResult, McpError> {
        let cmd = ClientCommand::UpdateQuickSettings {
            no_caching: p.no_caching,
            block_cookies: p.block_cookies,
            no_gzip: p.no_gzip,
        };
        match with_daemon_conn(&self.daemon_conn, &cmd).await {
            Ok(()) => {
                let mut active = Vec::new();
                if p.no_caching {
                    active.push("No Caching");
                }
                if p.block_cookies {
                    active.push("Block Cookies");
                }
                if p.no_gzip {
                    active.push("No Gzip");
                }
                if active.is_empty() {
                    tool_ok("All quick settings disabled.")
                } else {
                    tool_ok(format!("Quick settings active: {}", active.join(", ")))
                }
            }
            Err(e) => tool_error(format!("Failed to update quick settings: {}", e)),
        }
    }

    #[tool(
        description = "Configure client certificate for mTLS (mutual TLS). The proxy will present this certificate when connecting to upstream servers that require client authentication."
    )]
    pub(crate) async fn update_client_certificate(
        &self,
        Parameters(p): Parameters<UpdateClientCertificateParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = if p.enabled {
            let Some(cert_path) = p.cert_path else {
                return tool_error("cert_path is required when enabling client certificate.");
            };
            let Some(key_path) = p.key_path else {
                return tool_error("key_path is required when enabling client certificate.");
            };
            Some(proxy_daemon::ClientCertConfig {
                cert_path,
                key_path,
                enabled: true,
                domain_certs: Vec::new(),
            })
        } else {
            None
        };
        let cmd = ClientCommand::UpdateClientCertificate { config };
        match with_daemon_conn(&self.daemon_conn, &cmd).await {
            Ok(()) => {
                if p.enabled {
                    tool_ok("Client certificate enabled for mTLS.")
                } else {
                    tool_ok("Client certificate disabled.")
                }
            }
            Err(e) => tool_error(format!("Failed to update client certificate: {}", e)),
        }
    }

    #[tool(
        description = "Configure SSL proxying list. In blacklist mode (default), all domains are intercepted except listed ones. In whitelist mode, only listed domains are intercepted."
    )]
    pub(crate) async fn update_ssl_proxying_list(
        &self,
        Parameters(p): Parameters<UpdateSslProxyingListParams>,
    ) -> Result<CallToolResult, McpError> {
        let mode = match p.mode.to_lowercase().as_str() {
            "whitelist" => proxy_daemon::SslProxyingMode::Whitelist,
            _ => proxy_daemon::SslProxyingMode::Blacklist,
        };
        let entries: Vec<proxy_daemon::SslProxyingEntry> = p
            .entries
            .into_iter()
            .map(|e| proxy_daemon::SslProxyingEntry {
                pattern: e.pattern,
                enabled: e.enabled,
            })
            .collect();
        let count = entries.len();
        let cmd = ClientCommand::UpdateSslProxyingList {
            mode: mode.clone(),
            entries,
        };
        match with_daemon_conn(&self.daemon_conn, &cmd).await {
            Ok(()) => tool_ok(format!(
                "SSL proxying list updated ({:?} mode, {} entries).",
                mode, count
            )),
            Err(e) => tool_error(format!("Failed to update SSL proxying list: {}", e)),
        }
    }
}
