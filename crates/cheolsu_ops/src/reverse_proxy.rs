use proxy_daemon::ReverseProxyRule;

use crate::context::OpsContext;
use crate::helpers::{add_and_sync, list_items, remove_and_sync};
use crate::id::next_reverse_proxy_id;
use crate::params::*;
use crate::result::OpResult;

pub fn list_reverse_proxy_rules(ctx: &OpsContext) -> OpResult {
    list_items(
        &ctx.store.reverse_proxy_rules,
        "reverse proxy rules",
        "No reverse proxy rules configured.",
    )
}

pub async fn add_reverse_proxy_rule(ctx: &OpsContext, p: AddReverseProxyRuleParams) -> OpResult {
    let backend_scheme = p.backend_scheme.unwrap_or_else(|| "http".to_string());
    if backend_scheme != "http" && backend_scheme != "https" {
        return OpResult::err(format!(
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
        &ctx.store.reverse_proxy_rules,
        rule,
        &id,
        "Reverse proxy rule",
        || ctx.send_reverse_proxy_rules(),
    )
    .await
}

pub async fn remove_reverse_proxy_rule(
    ctx: &OpsContext,
    p: RemoveReverseProxyRuleParams,
) -> OpResult {
    remove_and_sync(
        &ctx.store.reverse_proxy_rules,
        &p.id,
        |r| &r.id,
        "Reverse proxy rule",
        || ctx.send_reverse_proxy_rules(),
    )
    .await
}
