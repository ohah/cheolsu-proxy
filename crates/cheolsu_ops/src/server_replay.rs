use proxy_daemon::ServerReplayEntry;

use crate::context::OpsContext;
use crate::helpers::{find_transaction_by_id, remove_and_sync};
use crate::id::next_server_replay_id;
use crate::params::*;
use crate::result::OpResult;

pub fn list_server_replay(ctx: &OpsContext) -> OpResult {
    let entries = ctx.store.server_replay_entries.lock();
    if entries.is_empty() {
        return OpResult::ok("No server replay entries configured.");
    }
    let list: Vec<String> = entries
        .iter()
        .map(|e| {
            format!(
                "  [{}] {} {} → {} (body: {})",
                e.id,
                e.method,
                e.url,
                e.status,
                e.body
                    .as_ref()
                    .map(|b| format!("{} chars", b.len()))
                    .unwrap_or_else(|| "none".to_string()),
            )
        })
        .collect();
    OpResult::ok(format!(
        "{} server replay entries:\n\n{}",
        entries.len(),
        list.join("\n")
    ))
}

pub async fn add_server_replay(ctx: &OpsContext, p: AddServerReplayParams) -> OpResult {
    let entry = {
        let txns = ctx.store.transactions.lock();
        let Some(info) = find_transaction_by_id(&txns, &p.transaction_id) else {
            return OpResult::err(format!("Transaction '{}' not found.", p.transaction_id));
        };

        let Some(req) = &info.request else {
            return OpResult::err("Transaction has no request data.");
        };

        let Some(res) = &info.response else {
            return OpResult::err(
                "Transaction has no response data. Only completed transactions can be added to server replay.",
            );
        };

        let headers: std::collections::HashMap<String, String> = res
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
            .collect();

        let body = if res.body_size() > 0 && res.data_type().is_text_based() {
            res.file_path()
                .as_ref()
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .or_else(|| res.body().and_then(|b| String::from_utf8(b.to_vec()).ok()))
        } else {
            None
        };

        ServerReplayEntry {
            id: next_server_replay_id(),
            method: req.method().to_string(),
            url: req.uri().to_string(),
            status: res.status().as_u16(),
            headers,
            body,
        }
    };

    let id = entry.id.clone();
    let url = entry.url.clone();
    ctx.store.server_replay_entries.lock().push(entry);

    match ctx.send_server_replay().await {
        Ok(()) => OpResult::ok(format!("Server replay entry '{}' added for {}.", id, url)),
        Err(e) => OpResult::err(format!(
            "Entry added locally but failed to sync with daemon: {}",
            e
        )),
    }
}

pub async fn remove_server_replay(ctx: &OpsContext, p: RemoveServerReplayParams) -> OpResult {
    remove_and_sync(
        &ctx.store.server_replay_entries,
        &p.id,
        |e| &e.id,
        "Server replay entry",
        || ctx.send_server_replay(),
    )
    .await
}

pub async fn clear_server_replay(ctx: &OpsContext) -> OpResult {
    ctx.store.server_replay_entries.lock().clear();
    match ctx.send_server_replay().await {
        Ok(()) => OpResult::ok("All server replay entries cleared."),
        Err(e) => OpResult::err(format!(
            "Entries cleared locally but failed to sync with daemon: {}",
            e
        )),
    }
}
