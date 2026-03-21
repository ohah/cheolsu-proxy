use proxy_daemon::{InterceptRule, SessionFile};

use crate::context::OpsContext;
use crate::params::*;
use crate::result::OpResult;

pub fn save_session(ctx: &OpsContext, p: SaveSessionParams) -> OpResult {
    let path = proxy_daemon::ensure_extension(&p.path);

    let transactions: Vec<proxy_v2_models::RequestInfo> = {
        let txns = ctx.store.transactions.lock();
        if let Some(ref filter) = p.filter {
            let filter_lower = filter.to_lowercase();
            txns.iter()
                .filter(|info| {
                    info.request
                        .as_ref()
                        .map(|req| req.uri().to_string().to_lowercase().contains(&filter_lower))
                        .unwrap_or(false)
                })
                .cloned()
                .collect()
        } else {
            txns.iter().cloned().collect()
        }
    };

    let ws_messages: Vec<proxy_v2_models::WsMessageInfo> = {
        let guard = ctx.store.ws_messages.lock();
        guard.iter().cloned().collect()
    };
    let rules: Vec<InterceptRule> = {
        let guard = ctx.store.rules.lock();
        guard.clone()
    };

    let mut session = SessionFile::from_traffic(0, &transactions, &ws_messages, &rules, &[], None);

    if let Some(name) = p.name {
        session.metadata.name = Some(name);
    }
    if let Some(desc) = p.description {
        session.metadata.description = Some(desc);
    }

    let file_path = std::path::Path::new(&path);
    if let Some(parent) = file_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return OpResult::err(format!("Failed to create directory: {}", e));
        }
    }

    match session.save(file_path) {
        Ok(()) => OpResult::ok(format!(
            "Session saved to '{}' ({} transactions, {} WebSocket messages).",
            path,
            transactions.len(),
            ws_messages.len(),
        )),
        Err(e) => OpResult::err(format!("Failed to save session: {}", e)),
    }
}

pub fn load_session(ctx: &OpsContext, p: LoadSessionParams) -> OpResult {
    let file_path = std::path::Path::new(&p.path);
    if !file_path.exists() {
        return OpResult::err(format!("File not found: {}", p.path));
    }

    let is_har = p.path.to_lowercase().ends_with(".har");

    let (transactions, ws_messages, rules) = if is_har {
        match proxy_daemon::import_har_file(file_path) {
            Ok(txns) => (txns, Vec::new(), Vec::new()),
            Err(e) => return OpResult::err(format!("Failed to import HAR file: {}", e)),
        }
    } else {
        match SessionFile::load(file_path) {
            Ok(session) => {
                let txns = session.extract_transactions();
                let ws = session.websocket_messages;
                let rules = session.intercept_rules;
                (txns, ws, rules)
            }
            Err(e) => return OpResult::err(format!("Failed to load session: {}", e)),
        }
    };

    let txn_count = transactions.len();
    let ws_count = ws_messages.len();
    let rule_count = rules.len();

    if !p.append {
        ctx.store.transactions.lock().clear();
        ctx.store.ws_messages.lock().clear();
        ctx.store.ws_connections.lock().clear();
    }

    {
        let mut txns = ctx.store.transactions.lock();
        for txn in transactions {
            txns.push_back(txn);
        }
    }
    {
        let mut msgs = ctx.store.ws_messages.lock();
        for msg in ws_messages {
            msgs.push_back(msg);
        }
    }

    if !rules.is_empty() {
        let mut current_rules = ctx.store.rules.lock();
        for rule in rules {
            if !current_rules.iter().any(|r| r.id == rule.id) {
                current_rules.push(rule);
            }
        }
    }

    let mode = if p.append { "appended" } else { "loaded" };
    let format_name = if is_har { "HAR" } else { "session" };

    OpResult::ok(format!(
        "{} {} from '{}': {} transactions, {} WebSocket messages, {} rules.",
        format_name, mode, p.path, txn_count, ws_count, rule_count,
    ))
}
