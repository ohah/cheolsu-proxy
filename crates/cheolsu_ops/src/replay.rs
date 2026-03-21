use crate::context::OpsContext;
use crate::helpers::{
    build_replay_client, compute_time_stats, find_transaction_by_id, format_size, truncate_text,
};
use crate::params::*;
use crate::result::OpResult;

pub async fn replay_request(p: ReplayRequestParams) -> OpResult {
    let client = match build_replay_client() {
        Ok(c) => c,
        Err(e) => return OpResult::err(e),
    };

    let method: reqwest::Method = match p.method.parse() {
        Ok(m) => m,
        Err(e) => return OpResult::err(format!("Invalid HTTP method: {}", e)),
    };

    let mut builder = client.request(method, &p.url);
    if let Some(headers) = p.headers {
        for (k, v) in headers {
            builder = builder.header(k, v);
        }
    }
    if let Some(body) = p.body {
        builder = builder.body(body);
    }

    let start = std::time::Instant::now();
    let response = match builder.send().await {
        Ok(r) => r,
        Err(e) => return OpResult::err(format!("Request failed: {}", e)),
    };
    let elapsed = start.elapsed();

    let status = response.status().as_u16();
    let headers: Vec<String> = response
        .headers()
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v.to_str().unwrap_or("<binary>")))
        .collect();
    let body_bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => return OpResult::err(format!("Failed to read response body: {}", e)),
    };

    let body_text = String::from_utf8(body_bytes.to_vec())
        .unwrap_or_else(|_| format!("<binary, {} bytes>", body_bytes.len()));
    let body_display = truncate_text(body_text, 10000);

    OpResult::ok(format!(
        "## Response\nStatus: {}\nTime: {:.0?}\nSize: {}\n\n### Headers\n```\n{}\n```\n\n### Body\n```\n{}\n```",
        status,
        elapsed,
        format_size(body_bytes.len()),
        headers.join("\n"),
        body_display,
    ))
}

pub async fn replay_sequence(ctx: &OpsContext, p: ReplaySequenceParams) -> OpResult {
    if p.transaction_ids.is_empty() {
        return OpResult::err("At least one transaction ID is required.");
    }

    let requests: Vec<(String, String, http::HeaderMap, Option<Vec<u8>>)> = {
        let txns = ctx.store.transactions.lock();
        let mut reqs = Vec::new();
        for id in &p.transaction_ids {
            let Some(info) = find_transaction_by_id(&txns, id) else {
                return OpResult::err(format!("Transaction '{}' not found.", id));
            };
            let Some(req) = &info.request else {
                return OpResult::err(format!("Transaction '{}' has no request data.", id));
            };
            let body = if req.body_size() > 0 {
                req.file_path()
                    .as_ref()
                    .and_then(|p| std::fs::read(p).ok())
                    .or_else(|| req.body().map(|b| b.to_vec()))
            } else {
                None
            };
            reqs.push((
                req.method().to_string(),
                req.uri().to_string(),
                req.headers().clone(),
                body,
            ));
        }
        reqs
    };

    let client = match build_replay_client() {
        Ok(c) => c,
        Err(e) => return OpResult::err(e),
    };

    let delay = std::time::Duration::from_millis(p.delay_ms.unwrap_or(0));
    let mut results = Vec::new();

    for (i, (method, url, headers, body)) in requests.into_iter().enumerate() {
        if i > 0 && !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }

        let method: reqwest::Method = match method.parse() {
            Ok(m) => m,
            Err(e) => {
                results.push(format!("[{}] {} {} → ERROR: {}", i + 1, method, url, e));
                continue;
            }
        };

        let mut builder = client.request(method.clone(), &url);
        for (k, v) in headers.iter() {
            if let Ok(v_str) = v.to_str() {
                builder = builder.header(k.as_str(), v_str);
            }
        }
        if let Some(body) = body {
            builder = builder.body(body);
        }

        let start = std::time::Instant::now();
        match builder.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let size = resp
                    .content_length()
                    .map(|l| l as usize)
                    .unwrap_or_default();
                let elapsed = start.elapsed();
                results.push(format!(
                    "[{}] {} {} → {} ({}) {:.0?}",
                    i + 1,
                    method,
                    url,
                    status,
                    format_size(size),
                    elapsed,
                ));
            }
            Err(e) => {
                results.push(format!("[{}] {} {} → ERROR: {}", i + 1, method, url, e));
            }
        }
    }

    OpResult::ok(format!(
        "Replayed {} requests:\n\n{}",
        results.len(),
        results.join("\n")
    ))
}

pub async fn advanced_repeat(p: AdvancedRepeatParams) -> OpResult {
    let iterations = p.iterations.unwrap_or(10) as usize;
    let concurrency = p.concurrency.unwrap_or(1).max(1) as usize;
    let delay_ms = p.delay_ms.unwrap_or(0);

    if iterations == 0 {
        return OpResult::err("iterations must be greater than 0.");
    }
    if iterations > 1000 {
        return OpResult::err("iterations must be 1000 or less.");
    }

    let method: reqwest::Method = match p.method.parse() {
        Ok(m) => m,
        Err(e) => return OpResult::err(format!("Invalid HTTP method: {}", e)),
    };

    let client = match build_replay_client() {
        Ok(c) => c,
        Err(e) => return OpResult::err(e),
    };

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));
    let results = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));

    let mut handles = Vec::new();
    let wall_start = std::time::Instant::now();

    for _ in 0..iterations {
        let sem = semaphore.clone();
        let client = client.clone();
        let method = method.clone();
        let url = p.url.clone();
        let headers = p.headers.clone();
        let body = p.body.clone();
        let res = results.clone();

        let handle = tokio::spawn(async move {
            let _permit = match sem.acquire().await {
                Ok(permit) => permit,
                Err(_) => return,
            };

            if delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }

            let mut builder = client.request(method, &url);
            if let Some(ref headers) = headers {
                for (k, v) in headers {
                    builder = builder.header(k.as_str(), v.as_str());
                }
            }
            if let Some(ref body) = body {
                builder = builder.body(body.clone());
            }

            let start = std::time::Instant::now();
            let result = builder.send().await;
            let elapsed = start.elapsed();

            let mut guard = res.lock().await;
            match result {
                Ok(resp) => guard.push((true, resp.status().as_u16(), elapsed)),
                Err(_) => guard.push((false, 0u16, elapsed)),
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    let wall_elapsed = wall_start.elapsed();
    let results = results.lock().await;
    let success_count = results.iter().filter(|(ok, _, _)| *ok).count();
    let failure_count = results.len() - success_count;

    let times: Vec<f64> = results
        .iter()
        .map(|(_, _, d)| d.as_secs_f64() * 1000.0)
        .collect();
    let (avg_ms, min_ms, max_ms) = compute_time_stats(&times);
    let wall_secs = wall_elapsed.as_secs_f64();
    let rps = if wall_secs > 0.0 {
        iterations as f64 / wall_secs
    } else {
        0.0
    };

    let mut status_counts: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();
    for (ok, status, _) in results.iter() {
        if *ok {
            *status_counts.entry(*status).or_insert(0) += 1;
        }
    }
    let status_dist: Vec<String> = {
        let mut entries: Vec<_> = status_counts.into_iter().collect();
        entries.sort_by_key(|(s, _)| *s);
        entries
            .iter()
            .map(|(s, c)| format!("  {} → {} times", s, c))
            .collect()
    };

    OpResult::ok(format!(
        "## Advanced Repeat Results\n\
         Target: {} {}\n\
         Iterations: {} (concurrency: {})\n\n\
         ### Summary\n\
         Success: {}\n\
         Failure: {}\n\
         Avg: {:.1}ms\n\
         Min: {:.1}ms\n\
         Max: {:.1}ms\n\
         RPS: {:.1}\n\n\
         ### Status Distribution\n{}",
        p.method,
        p.url,
        iterations,
        concurrency,
        success_count,
        failure_count,
        avg_ms,
        min_ms,
        max_ms,
        rps,
        status_dist.join("\n"),
    ))
}
