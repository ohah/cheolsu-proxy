use proxy_daemon::{
    diff_headers, diff_json, diff_text, is_text_data_type, BodyDiff, TrafficDiff,
    TransactionPartDiff,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct DiffTransactionData {
    pub method: Option<String>,
    pub uri: Option<String>,
    pub status: Option<u16>,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub body_size: usize,
    pub data_type: Option<String>,
}

fn diff_transaction_part(
    a: &DiffTransactionData,
    b: &DiffTransactionData,
    is_request: bool,
) -> Option<TransactionPartDiff> {
    let method_diff = if is_request {
        match (&a.method, &b.method) {
            (Some(ma), Some(mb)) if ma != mb => Some((ma.clone(), mb.clone())),
            _ => None,
        }
    } else {
        None
    };

    let url_diff = if is_request {
        match (&a.uri, &b.uri) {
            (Some(ua), Some(ub)) if ua != ub => Some((ua.clone(), ub.clone())),
            _ => None,
        }
    } else {
        None
    };

    let status_diff = if !is_request {
        match (a.status, b.status) {
            (Some(sa), Some(sb)) if sa != sb => Some((sa, sb)),
            _ => None,
        }
    } else {
        None
    };

    let header_diffs = diff_headers(&a.headers, &b.headers);

    let body_diff = compute_body_diff_from_strings(
        a.body.as_deref(),
        b.body.as_deref(),
        a.body_size,
        b.body_size,
        a.data_type.as_deref(),
        b.data_type.as_deref(),
    );

    if method_diff.is_none()
        && url_diff.is_none()
        && status_diff.is_none()
        && header_diffs.is_empty()
        && body_diff.is_none()
    {
        None
    } else {
        Some(TransactionPartDiff {
            method_diff,
            url_diff,
            status_diff,
            header_diffs,
            body_diff,
        })
    }
}

#[tauri::command]
pub(crate) async fn diff_transactions(
    transaction_a: DiffTransactionData,
    transaction_b: DiffTransactionData,
) -> Result<TrafficDiff, String> {
    let request_diff = diff_transaction_part(&transaction_a, &transaction_b, true);
    let response_diff = diff_transaction_part(&transaction_a, &transaction_b, false);

    Ok(TrafficDiff {
        request_diff,
        response_diff,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct DiffTransactionPair {
    pub request: Option<DiffTransactionData>,
    pub response: Option<DiffTransactionData>,
}

#[tauri::command]
pub(crate) async fn diff_transaction_pairs(
    pair_a: DiffTransactionPair,
    pair_b: DiffTransactionPair,
) -> Result<TrafficDiff, String> {
    let request_diff = match (&pair_a.request, &pair_b.request) {
        (Some(req_a), Some(req_b)) => diff_transaction_part(req_a, req_b, true),
        _ => None,
    };

    let response_diff = match (&pair_a.response, &pair_b.response) {
        (Some(res_a), Some(res_b)) => diff_transaction_part(res_a, res_b, false),
        _ => None,
    };

    Ok(TrafficDiff {
        request_diff,
        response_diff,
    })
}

fn compute_body_diff_from_strings(
    body_a: Option<&str>,
    body_b: Option<&str>,
    size_a: usize,
    size_b: usize,
    data_type_a: Option<&str>,
    data_type_b: Option<&str>,
) -> Option<BodyDiff> {
    let text_a = body_a.unwrap_or("");
    let text_b = body_b.unwrap_or("");

    if text_a == text_b {
        return None;
    }

    let is_json = matches!(data_type_a, Some("Json" | "GraphQL"))
        && matches!(data_type_b, Some("Json" | "GraphQL"));

    if is_json {
        if let (Ok(json_a), Ok(json_b)) = (
            serde_json::from_str::<serde_json::Value>(text_a),
            serde_json::from_str::<serde_json::Value>(text_b),
        ) {
            return Some(diff_json(&json_a, &json_b));
        }
    }

    let is_text = data_type_a.map(|t| is_text_data_type(t)).unwrap_or(true)
        && data_type_b.map(|t| is_text_data_type(t)).unwrap_or(true);

    if is_text && !text_a.is_empty() && !text_b.is_empty() {
        return Some(diff_text(text_a, text_b));
    }

    Some(BodyDiff::Binary {
        old_size: size_a,
        new_size: size_b,
    })
}
