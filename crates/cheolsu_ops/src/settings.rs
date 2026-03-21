use proxy_daemon::ClientCommand;

use crate::context::OpsContext;
use crate::helpers::with_daemon_conn;
use crate::params::*;
use crate::result::OpResult;

pub async fn update_upstream_proxy(ctx: &OpsContext, p: UpdateUpstreamProxyParams) -> OpResult {
    let config = if p.enabled {
        let Some(host) = p.host else {
            return OpResult::err("host is required when enabling upstream proxy.");
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
    match with_daemon_conn(&ctx.daemon_conn, &cmd).await {
        Ok(()) => {
            if p.enabled {
                OpResult::ok("Upstream proxy enabled.")
            } else {
                OpResult::ok("Upstream proxy disabled.")
            }
        }
        Err(e) => OpResult::err(format!("Failed to update upstream proxy: {}", e)),
    }
}

pub async fn update_throttle(ctx: &OpsContext, p: UpdateThrottleParams) -> OpResult {
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
    match with_daemon_conn(&ctx.daemon_conn, &cmd).await {
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
                OpResult::ok(format!("Throttle enabled ({})", parts.join(", ")))
            } else {
                OpResult::ok("Throttle disabled.")
            }
        }
        Err(e) => OpResult::err(format!("Failed to update throttle: {}", e)),
    }
}

pub async fn update_proxy_auth(ctx: &OpsContext, p: UpdateProxyAuthParams) -> OpResult {
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
    match with_daemon_conn(&ctx.daemon_conn, &cmd).await {
        Ok(()) => {
            if p.enabled {
                OpResult::ok("Proxy authentication enabled.")
            } else {
                OpResult::ok("Proxy authentication disabled.")
            }
        }
        Err(e) => OpResult::err(format!("Failed to update proxy auth: {}", e)),
    }
}

pub async fn update_connection_strategy(
    ctx: &OpsContext,
    p: UpdateConnectionStrategyParams,
) -> OpResult {
    let strategy = p.strategy.to_lowercase();
    if !["lazy", "eager", "eager_with_fallback"].contains(&strategy.as_str()) {
        return OpResult::err("Invalid strategy. Use: 'lazy', 'eager', or 'eager_with_fallback'.");
    }
    let cmd = ClientCommand::UpdateConnectionStrategy {
        strategy: strategy.clone(),
    };
    match with_daemon_conn(&ctx.daemon_conn, &cmd).await {
        Ok(()) => OpResult::ok(format!("Connection strategy set to '{}'.", strategy)),
        Err(e) => OpResult::err(format!("Failed to update connection strategy: {}", e)),
    }
}

pub async fn update_quick_settings(ctx: &OpsContext, p: UpdateQuickSettingsParams) -> OpResult {
    let cmd = ClientCommand::UpdateQuickSettings {
        no_caching: p.no_caching,
        block_cookies: p.block_cookies,
        no_gzip: p.no_gzip,
    };
    match with_daemon_conn(&ctx.daemon_conn, &cmd).await {
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
                OpResult::ok("All quick settings disabled.")
            } else {
                OpResult::ok(format!("Quick settings active: {}", active.join(", ")))
            }
        }
        Err(e) => OpResult::err(format!("Failed to update quick settings: {}", e)),
    }
}

pub async fn update_client_certificate(
    ctx: &OpsContext,
    p: UpdateClientCertificateParams,
) -> OpResult {
    let config = if p.enabled {
        let Some(cert_path) = p.cert_path else {
            return OpResult::err("cert_path is required when enabling client certificate.");
        };
        let Some(key_path) = p.key_path else {
            return OpResult::err("key_path is required when enabling client certificate.");
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
    match with_daemon_conn(&ctx.daemon_conn, &cmd).await {
        Ok(()) => {
            if p.enabled {
                OpResult::ok("Client certificate enabled for mTLS.")
            } else {
                OpResult::ok("Client certificate disabled.")
            }
        }
        Err(e) => OpResult::err(format!("Failed to update client certificate: {}", e)),
    }
}

pub async fn update_ssl_proxying_list(
    ctx: &OpsContext,
    p: UpdateSslProxyingListParams,
) -> OpResult {
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
    match with_daemon_conn(&ctx.daemon_conn, &cmd).await {
        Ok(()) => OpResult::ok(format!(
            "SSL proxying list updated ({:?} mode, {} entries).",
            mode, count
        )),
        Err(e) => OpResult::err(format!("Failed to update SSL proxying list: {}", e)),
    }
}
