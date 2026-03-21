use proxy_daemon::analytics::TrafficAnalytics;

use crate::context::OpsContext;
use crate::params::*;
use crate::result::OpResult;

fn collect_entries(ctx: &OpsContext) -> Vec<proxy_v2_models::RequestInfo> {
    ctx.store.transactions.lock().iter().cloned().collect()
}

pub fn analyze_performance(ctx: &OpsContext, p: AnalyzePerformanceParams) -> OpResult {
    let entries = collect_entries(ctx);
    let threshold = p.threshold_ms.unwrap_or(1000);
    let limit = p.limit.unwrap_or(20);
    let report = TrafficAnalytics::slow_requests(&entries, threshold, limit);
    let json = serde_json::to_string_pretty(&report)
        .unwrap_or_else(|e| format!("Serialization error: {}", e));
    OpResult::ok(format!(
        "Performance analysis ({} total transactions, threshold {}ms):\n\n```json\n{}\n```",
        entries.len(),
        threshold,
        json
    ))
}

pub fn analyze_errors(ctx: &OpsContext) -> OpResult {
    let entries = collect_entries(ctx);
    let report = TrafficAnalytics::error_analysis(&entries);
    let json = serde_json::to_string_pretty(&report)
        .unwrap_or_else(|e| format!("Serialization error: {}", e));
    OpResult::ok(format!(
        "Error analysis ({} total transactions):\n\n```json\n{}\n```",
        entries.len(),
        json
    ))
}

pub fn analyze_endpoints(ctx: &OpsContext, p: AnalyzeEndpointsParams) -> OpResult {
    let entries = collect_entries(ctx);
    let mut stats = TrafficAnalytics::endpoint_stats(&entries);

    match p.sort_by.as_deref() {
        Some("duration") => {
            stats.sort_by(|a, b| {
                b.avg_duration_ms
                    .partial_cmp(&a.avg_duration_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        Some("error_rate") => {
            stats.sort_by(|a, b| {
                b.error_rate
                    .partial_cmp(&a.error_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        _ => {}
    }

    let limit = p.limit.unwrap_or(30);
    stats.truncate(limit);

    let json = serde_json::to_string_pretty(&stats)
        .unwrap_or_else(|e| format!("Serialization error: {}", e));
    OpResult::ok(format!(
        "Endpoint statistics ({} endpoints from {} transactions):\n\n```json\n{}\n```",
        stats.len(),
        entries.len(),
        json
    ))
}

pub fn detect_duplicates(ctx: &OpsContext, p: DetectDuplicatesParams) -> OpResult {
    let entries = collect_entries(ctx);
    let window = p.window_ms.unwrap_or(3000);
    let dupes = TrafficAnalytics::duplicate_requests(&entries, window);
    let json = serde_json::to_string_pretty(&dupes)
        .unwrap_or_else(|e| format!("Serialization error: {}", e));

    if dupes.is_empty() {
        OpResult::ok(format!(
            "No duplicate requests detected within {}ms window ({} transactions analyzed).",
            window,
            entries.len()
        ))
    } else {
        OpResult::ok(format!(
            "Found {} duplicate request groups (window {}ms, {} transactions):\n\n```json\n{}\n```",
            dupes.len(),
            window,
            entries.len(),
            json
        ))
    }
}

pub fn detect_n_plus_one(ctx: &OpsContext) -> OpResult {
    let entries = collect_entries(ctx);
    let patterns = TrafficAnalytics::n_plus_one_detection(&entries);
    let json = serde_json::to_string_pretty(&patterns)
        .unwrap_or_else(|e| format!("Serialization error: {}", e));

    if patterns.is_empty() {
        OpResult::ok(format!(
            "No N+1 query patterns detected ({} transactions analyzed).",
            entries.len()
        ))
    } else {
        OpResult::ok(format!(
            "Found {} potential N+1 patterns ({} transactions):\n\n```json\n{}\n```",
            patterns.len(),
            entries.len(),
            json
        ))
    }
}

pub fn analyze_traffic_timeline(ctx: &OpsContext, p: AnalyzeTimelineParams) -> OpResult {
    let entries = collect_entries(ctx);
    let bucket_seconds = p.bucket_seconds.unwrap_or(60);
    let timeline = TrafficAnalytics::traffic_timeline(&entries, bucket_seconds);
    let json = serde_json::to_string_pretty(&timeline)
        .unwrap_or_else(|e| format!("Serialization error: {}", e));
    OpResult::ok(format!(
        "Traffic timeline ({} buckets of {}s, {} transactions):\n\n```json\n{}\n```",
        timeline.len(),
        bucket_seconds,
        entries.len(),
        json
    ))
}

pub fn analyze_full(ctx: &OpsContext) -> OpResult {
    let entries = collect_entries(ctx);
    let report = TrafficAnalytics::full_report(&entries);
    let json = serde_json::to_string_pretty(&report)
        .unwrap_or_else(|e| format!("Serialization error: {}", e));
    OpResult::ok(format!(
        "Full traffic analysis ({} transactions):\n\n```json\n{}\n```",
        entries.len(),
        json
    ))
}
