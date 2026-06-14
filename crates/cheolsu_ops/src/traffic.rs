use proxy_daemon::{format_diff_text, TrafficDiff, TransactionPartDiff};
use proxy_v2_models::WsDirection;

use crate::context::OpsContext;
use crate::diff::diff_part;
use crate::helpers::{find_transaction_by_id, format_size, read_body_text};
use crate::params::*;
use crate::result::OpResult;

pub fn search_traffic(ctx: &OpsContext, p: SearchTrafficParams) -> OpResult {
    let txns = ctx.store.transactions.lock();
    let limit = p.limit.unwrap_or(50);

    let results: Vec<String> = txns
        .iter()
        .rev()
        .filter(|info| {
            let Some(req) = &info.request else {
                return false;
            };
            let uri = req.uri().to_string();

            if let Some(ref host) = p.host {
                if !uri.to_lowercase().contains(&host.to_lowercase()) {
                    return false;
                }
            }
            if let Some(ref method) = p.method {
                if !req.method().as_str().eq_ignore_ascii_case(method) {
                    return false;
                }
            }
            if let Some(status) = p.status {
                match &info.response {
                    Some(res) if res.status().as_u16() == status => {}
                    _ => return false,
                }
            }
            if let Some(ref path) = p.path {
                if !uri.to_lowercase().contains(&path.to_lowercase()) {
                    return false;
                }
            }
            true
        })
        .take(limit)
        .filter_map(|info| {
            let req = info.request.as_ref()?;
            let status = info
                .response
                .as_ref()
                .map(|r| r.status().as_u16().to_string())
                .unwrap_or_else(|| "pending".to_string());
            let size = info
                .response
                .as_ref()
                .map(|r| format_size(r.body_size()))
                .unwrap_or_default();
            let dtype = info
                .response
                .as_ref()
                .map(|r| format!("{:?}", r.data_type()))
                .unwrap_or_default();
            Some(format!(
                "[{}] {} {} → {} ({}) {}",
                req.id(),
                req.method(),
                req.uri(),
                status,
                size,
                dtype,
            ))
        })
        .collect();

    if results.is_empty() {
        OpResult::ok("No matching transactions found.")
    } else {
        OpResult::ok(format!(
            "Found {} transactions (most recent first):\n\n{}",
            results.len(),
            results.join("\n")
        ))
    }
}

pub fn get_transaction(ctx: &OpsContext, p: GetTransactionParams) -> OpResult {
    let txns = ctx.store.transactions.lock();
    let Some(info) = find_transaction_by_id(&txns, &p.id) else {
        return OpResult::err(format!("Transaction '{}' not found.", p.id));
    };

    let mut out = String::new();

    if let Some(req) = &info.request {
        out.push_str(&format!(
            "## Request\n{} {} {:?}\n\n",
            req.method(),
            req.uri(),
            req.version()
        ));
        out.push_str("### Headers\n```\n");
        for (name, value) in req.headers().iter() {
            out.push_str(&format!(
                "{}: {}\n",
                name,
                value.to_str().unwrap_or("<binary>")
            ));
        }
        out.push_str("```\n\n");
        out.push_str(&format!(
            "### Body ({}, {:?})\n",
            format_size(req.body_size()),
            req.data_type()
        ));
        if req.body_size() == 0 {
            out.push_str("(empty)\n");
        } else {
            let body = read_body_text(req.file_path(), req.data_type());
            out.push_str(&format!("```\n{}\n```\n", body));
        }
    }

    if let Some(res) = &info.response {
        out.push_str(&format!(
            "\n## Response\n{} {:?}\n\n",
            res.status().as_u16(),
            res.version()
        ));
        out.push_str("### Headers\n```\n");
        for (name, value) in res.headers().iter() {
            out.push_str(&format!(
                "{}: {}\n",
                name,
                value.to_str().unwrap_or("<binary>")
            ));
        }
        out.push_str("```\n\n");
        out.push_str(&format!(
            "### Body ({}, {:?})\n",
            format_size(res.body_size()),
            res.data_type()
        ));
        if res.body_size() == 0 {
            out.push_str("(empty)\n");
        } else {
            let body = read_body_text(res.file_path(), res.data_type());
            out.push_str(&format!("```\n{}\n```\n", body));
        }
    }

    OpResult::ok(out)
}

pub fn get_websocket_messages(ctx: &OpsContext, p: GetWsMessagesParams) -> OpResult {
    let msgs = ctx.store.ws_messages.lock();
    let limit = p.limit.unwrap_or(100);

    let results: Vec<String> = msgs
        .iter()
        .rev()
        .filter(|msg| {
            p.connection_id.as_ref().map_or(true, |cid| {
                msg.connection_id
                    .to_lowercase()
                    .contains(&cid.to_lowercase())
            })
        })
        .take(limit)
        .map(|msg| {
            let dir = match msg.direction {
                WsDirection::ClientToServer => "→",
                WsDirection::ServerToClient => "←",
            };
            let payload = if msg.payload.len() > 200 {
                // UTF-8 문자 경계에서 자르기(멀티바이트 경계 슬라이싱 패닉 방지)
                let mut end = 200;
                while end > 0 && !msg.payload.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &msg.payload[..end])
            } else {
                msg.payload.clone()
            };
            format!(
                "{} {:?} ({} bytes) [{}]: {}",
                dir, msg.message_type, msg.size, msg.connection_id, payload,
            )
        })
        .collect();

    if results.is_empty() {
        OpResult::ok("No WebSocket messages captured.")
    } else {
        OpResult::ok(format!(
            "Found {} messages:\n\n{}",
            results.len(),
            results.join("\n\n")
        ))
    }
}

pub fn diff_transactions(ctx: &OpsContext, p: DiffTransactionsParams) -> OpResult {
    let txns = ctx.store.transactions.lock();

    let Some(txn_a) = find_transaction_by_id(&txns, &p.transaction_id_a) else {
        return OpResult::err(format!("Transaction '{}' not found.", p.transaction_id_a));
    };
    let Some(txn_b) = find_transaction_by_id(&txns, &p.transaction_id_b) else {
        return OpResult::err(format!("Transaction '{}' not found.", p.transaction_id_b));
    };

    let request_diff = match (&txn_a.request, &txn_b.request) {
        (Some(req_a), Some(req_b)) => {
            let method_diff = if req_a.method() != req_b.method() {
                Some((req_a.method().to_string(), req_b.method().to_string()))
            } else {
                None
            };
            let url_diff = if req_a.uri().to_string() != req_b.uri().to_string() {
                Some((req_a.uri().to_string(), req_b.uri().to_string()))
            } else {
                None
            };
            let (header_diffs, body_diff) = diff_part(
                req_a.headers(),
                req_b.headers(),
                req_a.body().map(|b| b.as_ref()),
                req_b.body().map(|b| b.as_ref()),
                req_a.body_size(),
                req_b.body_size(),
                req_a.file_path(),
                req_b.file_path(),
                req_a.data_type(),
                req_b.data_type(),
            );
            if method_diff.is_none()
                && url_diff.is_none()
                && header_diffs.is_empty()
                && body_diff.is_none()
            {
                None
            } else {
                Some(TransactionPartDiff {
                    method_diff,
                    url_diff,
                    status_diff: None,
                    header_diffs,
                    body_diff,
                })
            }
        }
        _ => None,
    };

    let response_diff = match (&txn_a.response, &txn_b.response) {
        (Some(res_a), Some(res_b)) => {
            let status_diff = if res_a.status() != res_b.status() {
                Some((res_a.status().as_u16(), res_b.status().as_u16()))
            } else {
                None
            };
            let (header_diffs, body_diff) = diff_part(
                res_a.headers(),
                res_b.headers(),
                res_a.body().map(|b| b.as_ref()),
                res_b.body().map(|b| b.as_ref()),
                res_a.body_size(),
                res_b.body_size(),
                res_a.file_path(),
                res_b.file_path(),
                res_a.data_type(),
                res_b.data_type(),
            );
            if status_diff.is_none() && header_diffs.is_empty() && body_diff.is_none() {
                None
            } else {
                Some(TransactionPartDiff {
                    method_diff: None,
                    url_diff: None,
                    status_diff,
                    header_diffs,
                    body_diff,
                })
            }
        }
        _ => None,
    };

    let diff = TrafficDiff {
        request_diff,
        response_diff,
    };

    OpResult::ok(format_diff_text(&diff))
}

pub async fn proxy_status(ctx: &OpsContext) -> OpResult {
    let connected = ctx.daemon_conn.lock().await.is_some();
    let txn_count = ctx.store.transactions.lock().len();
    let ws_msg_count = ctx.store.ws_messages.lock().len();
    let ws_conn_count = ctx.store.ws_connections.lock().len();
    let rule_count = ctx.store.rules.lock().len();
    let daemon_running = proxy_daemon::is_daemon_running().is_some();

    OpResult::ok(format!(
        "Daemon running: {}\nMCP connected: {}\nCaptured transactions: {}\nWebSocket connections: {}\nWebSocket messages: {}\nIntercept rules: {}",
        daemon_running, connected, txn_count, ws_conn_count, ws_msg_count, rule_count,
    ))
}

pub fn clear_traffic(ctx: &OpsContext) -> OpResult {
    ctx.store.transactions.lock().clear();
    ctx.store.ws_messages.lock().clear();
    ctx.store.ws_connections.lock().clear();
    OpResult::ok("All captured traffic cleared.")
}

pub fn generate_openapi_spec(ctx: &OpsContext, p: GenerateOpenApiParams) -> OpResult {
    let txns = ctx.store.transactions.lock();

    let filtered: Vec<proxy_v2_models::RequestInfo> = txns
        .iter()
        .filter(|info| {
            let Some(req) = &info.request else {
                return false;
            };
            let uri = req.uri().to_string();

            if let Some(ref host) = p.host {
                if !uri.to_lowercase().contains(&host.to_lowercase()) {
                    return false;
                }
            }
            if let Some(ref prefix) = p.path_prefix {
                let path = uri
                    .parse::<http::Uri>()
                    .ok()
                    .and_then(|u| u.path_and_query().map(|pq| pq.path().to_string()))
                    .unwrap_or_default();
                if !path.starts_with(prefix) {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    if filtered.is_empty() {
        return OpResult::ok("No matching transactions found. Capture some HTTP traffic first.");
    }

    let mut spec = proxy_v2_models::openapi::build_openapi_spec(&filtered);

    if let Some(title) = p.title {
        spec.info.title = title;
    }

    let output = serde_json::to_string_pretty(&spec)
        .unwrap_or_else(|e| format!("Serialization error: {}", e));

    OpResult::ok(format!(
        "Generated OpenAPI 3.0 spec from {} transactions:\n\n```json\n{}\n```",
        filtered.len(),
        output
    ))
}
