# Traffic Analytics

## Overview

Traffic Analytics analyzes captured HTTP traffic from multiple angles, automatically identifying performance bottlenecks, error patterns, and inefficient API calls. Instead of inspecting individual requests, extract patterns from all traffic to quickly discover issues.

---

## Analysis Reports

### Slow Request Analysis

Identifies requests exceeding a configured threshold and provides response time percentiles.

| Metric | Description |
| --- | --- |
| **P50** | Median response time |
| **P95** | 95th percentile response time |
| **P99** | 99th percentile response time |
| **Slow Request Count** | Number of requests exceeding threshold |

### Error Analysis

Classifies HTTP error responses (4xx, 5xx) by status code and endpoint.

- Overall error rate
- Status code distribution (e.g., 404 — 12, 500 — 3)
- Endpoints ranked by error count
- Recent error list

### Endpoint Statistics

Aggregates performance metrics per API endpoint.

| Metric | Description |
| --- | --- |
| **Method + Path** | API endpoint identifier |
| **Request Count** | Number of requests |
| **Avg Response Time** | Average duration |
| **P95 Response Time** | 95th percentile duration |
| **Error Rate** | Error response ratio |
| **Avg Response Size** | Response body size |

### Duplicate Request Detection

Detects repeated requests to the same URL within a time window. Identifies optimization opportunities to reduce unnecessary network calls.

### N+1 Query Detection

Detects N+1 patterns where 3 or more similar requests occur within a short time window. Discovers inefficient patterns like fetching detail data for each item in a list via individual API calls.

### Traffic Timeline

Displays request count, error count, and average response time over time. Visually identify traffic pattern changes and performance degradation periods.

### Domain Breakdown

Aggregates request count, errors, average response time, and total bytes sent/received per domain.

### Payload Size Analysis

Analyzes request/response body sizes to identify abnormally large payloads.

- Average/maximum request size
- Average/maximum response size
- Largest transaction list

### CORS Issue Detection

Automatically detects missing or misconfigured CORS headers.

### Mixed Content Warnings

Detects Mixed Content issues where HTTP resources are loaded from HTTPS pages.

---

## Full Report

Runs all analyses at once to generate a comprehensive report covering all captured traffic.

---

## Usage

### Desktop

Select **Analytics** from the sidebar to view analysis results in the dashboard.

### MCP

```
"Analyze slow requests"
"Show me endpoints with high error rates"
"Check for N+1 queries"
"Generate a full traffic analysis report"
```
