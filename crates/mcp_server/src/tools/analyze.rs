use proxy_daemon::analytics::TrafficAnalytics;
use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::tool_ok;
use crate::params::{
    AnalyzeEndpointsParams, AnalyzeErrorsParams, AnalyzeFullParams, AnalyzePerformanceParams,
    AnalyzeTimelineParams, DetectDuplicatesParams, DetectNPlus1Params,
};
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "Analyze slow HTTP requests. Returns requests exceeding a duration threshold, plus P50/P95/P99 latency percentiles across all captured traffic."
    )]
    pub(crate) async fn analyze_performance(
        &self,
        Parameters(p): Parameters<AnalyzePerformanceParams>,
    ) -> Result<CallToolResult, McpError> {
        let txns = self.store.transactions.lock();
        let entries: Vec<_> = txns.iter().cloned().collect();
        drop(txns);

        let threshold = p.threshold_ms.unwrap_or(1000);
        let limit = p.limit.unwrap_or(20);
        let report = TrafficAnalytics::slow_requests(&entries, threshold, limit);

        let json = serde_json::to_string_pretty(&report)
            .unwrap_or_else(|e| format!("Serialization error: {}", e));

        tool_ok(format!(
            "Performance analysis ({} total transactions, threshold {}ms):\n\n```json\n{}\n```",
            entries.len(),
            threshold,
            json
        ))
    }

    #[tool(
        description = "Analyze HTTP error rates. Returns error count, error rate percentage, breakdown by status code, error rates per endpoint, and recent errors."
    )]
    pub(crate) async fn analyze_errors(
        &self,
        #[allow(unused_variables)] Parameters(_p): Parameters<AnalyzeErrorsParams>,
    ) -> Result<CallToolResult, McpError> {
        let txns = self.store.transactions.lock();
        let entries: Vec<_> = txns.iter().cloned().collect();
        drop(txns);

        let report = TrafficAnalytics::error_analysis(&entries);

        let json = serde_json::to_string_pretty(&report)
            .unwrap_or_else(|e| format!("Serialization error: {}", e));

        tool_ok(format!(
            "Error analysis ({} total transactions):\n\n```json\n{}\n```",
            entries.len(),
            json
        ))
    }

    #[tool(
        description = "Get per-endpoint traffic statistics. Returns call frequency, average response time, P95 latency, error rate, and average response size for each endpoint."
    )]
    pub(crate) async fn analyze_endpoints(
        &self,
        Parameters(p): Parameters<AnalyzeEndpointsParams>,
    ) -> Result<CallToolResult, McpError> {
        let txns = self.store.transactions.lock();
        let entries: Vec<_> = txns.iter().cloned().collect();
        drop(txns);

        let mut stats = TrafficAnalytics::endpoint_stats(&entries);

        // 정렬
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
            _ => {} // 기본: count 순 (이미 정렬됨)
        }

        let limit = p.limit.unwrap_or(30);
        stats.truncate(limit);

        let json = serde_json::to_string_pretty(&stats)
            .unwrap_or_else(|e| format!("Serialization error: {}", e));

        tool_ok(format!(
            "Endpoint statistics ({} endpoints from {} transactions):\n\n```json\n{}\n```",
            stats.len(),
            entries.len(),
            json
        ))
    }

    #[tool(
        description = "Detect duplicate HTTP requests. Finds the same URL called multiple times within a short time window, which may indicate unnecessary re-fetching or missing caching."
    )]
    pub(crate) async fn detect_duplicates(
        &self,
        Parameters(p): Parameters<DetectDuplicatesParams>,
    ) -> Result<CallToolResult, McpError> {
        let txns = self.store.transactions.lock();
        let entries: Vec<_> = txns.iter().cloned().collect();
        drop(txns);

        let window = p.window_ms.unwrap_or(3000);
        let dupes = TrafficAnalytics::duplicate_requests(&entries, window);

        let json = serde_json::to_string_pretty(&dupes)
            .unwrap_or_else(|e| format!("Serialization error: {}", e));

        if dupes.is_empty() {
            tool_ok(format!(
                "No duplicate requests detected within {}ms window ({} transactions analyzed).",
                window,
                entries.len()
            ))
        } else {
            tool_ok(format!(
                "Found {} duplicate request groups (window {}ms, {} transactions):\n\n```json\n{}\n```",
                dupes.len(),
                window,
                entries.len(),
                json
            ))
        }
    }

    #[tool(
        description = "Detect N+1 query patterns. Identifies when the same parameterized endpoint (e.g., /api/users/{id}) is called many times in rapid succession, suggesting a missing batch/list endpoint."
    )]
    pub(crate) async fn detect_n_plus_one(
        &self,
        #[allow(unused_variables)] Parameters(_p): Parameters<DetectNPlus1Params>,
    ) -> Result<CallToolResult, McpError> {
        let txns = self.store.transactions.lock();
        let entries: Vec<_> = txns.iter().cloned().collect();
        drop(txns);

        let patterns = TrafficAnalytics::n_plus_one_detection(&entries);

        let json = serde_json::to_string_pretty(&patterns)
            .unwrap_or_else(|e| format!("Serialization error: {}", e));

        if patterns.is_empty() {
            tool_ok(format!(
                "No N+1 query patterns detected ({} transactions analyzed).",
                entries.len()
            ))
        } else {
            tool_ok(format!(
                "Found {} potential N+1 patterns ({} transactions):\n\n```json\n{}\n```",
                patterns.len(),
                entries.len(),
                json
            ))
        }
    }

    #[tool(
        description = "Get traffic timeline showing request volume, error count, and average latency over time buckets. Useful for identifying traffic spikes and performance degradation patterns."
    )]
    pub(crate) async fn analyze_traffic_timeline(
        &self,
        Parameters(p): Parameters<AnalyzeTimelineParams>,
    ) -> Result<CallToolResult, McpError> {
        let txns = self.store.transactions.lock();
        let entries: Vec<_> = txns.iter().cloned().collect();
        drop(txns);

        let bucket_seconds = p.bucket_seconds.unwrap_or(60);
        let timeline = TrafficAnalytics::traffic_timeline(&entries, bucket_seconds);

        let json = serde_json::to_string_pretty(&timeline)
            .unwrap_or_else(|e| format!("Serialization error: {}", e));

        tool_ok(format!(
            "Traffic timeline ({} buckets of {}s, {} transactions):\n\n```json\n{}\n```",
            timeline.len(),
            bucket_seconds,
            entries.len(),
            json
        ))
    }

    #[tool(
        description = "Run all traffic analyses at once: slow requests, errors, endpoint stats, duplicates, N+1 patterns, domain breakdown, payload sizes, CORS issues, and mixed content warnings. Returns a comprehensive summary report."
    )]
    pub(crate) async fn analyze_full(
        &self,
        #[allow(unused_variables)] Parameters(_p): Parameters<AnalyzeFullParams>,
    ) -> Result<CallToolResult, McpError> {
        let txns = self.store.transactions.lock();
        let entries: Vec<_> = txns.iter().cloned().collect();
        drop(txns);

        let report = TrafficAnalytics::full_report(&entries);

        let json = serde_json::to_string_pretty(&report)
            .unwrap_or_else(|e| format!("Serialization error: {}", e));

        tool_ok(format!(
            "Full traffic analysis ({} transactions):\n\n```json\n{}\n```",
            entries.len(),
            json
        ))
    }
}
