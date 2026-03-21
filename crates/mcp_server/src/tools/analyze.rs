use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::op_result_to_mcp;
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "Analyze slow HTTP requests. Returns requests exceeding a duration threshold, plus P50/P95/P99 latency percentiles across all captured traffic."
    )]
    pub(crate) async fn analyze_performance(
        &self,
        Parameters(p): Parameters<AnalyzePerformanceParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::analyze::analyze_performance(
            &self.ops_ctx(),
            p,
        ))
    }

    #[tool(
        description = "Analyze HTTP error rates. Returns error count, error rate percentage, breakdown by status code, error rates per endpoint, and recent errors."
    )]
    pub(crate) async fn analyze_errors(
        &self,
        #[allow(unused_variables)] Parameters(_p): Parameters<AnalyzeErrorsParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::analyze::analyze_errors(&self.ops_ctx()))
    }

    #[tool(
        description = "Get per-endpoint traffic statistics. Returns call frequency, average response time, P95 latency, error rate, and average response size for each endpoint."
    )]
    pub(crate) async fn analyze_endpoints(
        &self,
        Parameters(p): Parameters<AnalyzeEndpointsParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::analyze::analyze_endpoints(&self.ops_ctx(), p))
    }

    #[tool(
        description = "Detect duplicate HTTP requests. Finds the same URL called multiple times within a short time window, which may indicate unnecessary re-fetching or missing caching."
    )]
    pub(crate) async fn detect_duplicates(
        &self,
        Parameters(p): Parameters<DetectDuplicatesParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::analyze::detect_duplicates(&self.ops_ctx(), p))
    }

    #[tool(
        description = "Detect N+1 query patterns. Identifies when the same parameterized endpoint (e.g., /api/users/{id}) is called many times in rapid succession, suggesting a missing batch/list endpoint."
    )]
    pub(crate) async fn detect_n_plus_one(
        &self,
        #[allow(unused_variables)] Parameters(_p): Parameters<DetectNPlus1Params>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::analyze::detect_n_plus_one(&self.ops_ctx()))
    }

    #[tool(
        description = "Get traffic timeline showing request volume, error count, and average latency over time buckets. Useful for identifying traffic spikes and performance degradation patterns."
    )]
    pub(crate) async fn analyze_traffic_timeline(
        &self,
        Parameters(p): Parameters<AnalyzeTimelineParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::analyze::analyze_traffic_timeline(
            &self.ops_ctx(),
            p,
        ))
    }

    #[tool(
        description = "Run all traffic analyses at once: slow requests, errors, endpoint stats, duplicates, N+1 patterns, domain breakdown, payload sizes, CORS issues, and mixed content warnings. Returns a comprehensive summary report."
    )]
    pub(crate) async fn analyze_full(
        &self,
        #[allow(unused_variables)] Parameters(_p): Parameters<AnalyzeFullParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::analyze::analyze_full(&self.ops_ctx()))
    }
}
