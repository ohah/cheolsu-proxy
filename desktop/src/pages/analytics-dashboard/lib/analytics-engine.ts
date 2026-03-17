import type { HttpTransaction } from "@/entities/proxy";

// ── Report Types ──

export interface SummaryMetrics {
  totalRequests: number;
  errorRate: number;
  avgResponseTime: number;
  p95ResponseTime: number;
}

export interface SlowRequest {
  url: string;
  method: string;
  durationMs: number;
  timestamp: number;
  status: number | null;
}

export interface PerformanceReport {
  slowRequests: SlowRequest[];
  p50: number;
  p95: number;
  p99: number;
}

export interface ErrorStatusEntry {
  statusCode: number;
  count: number;
}

export interface ErrorEndpoint {
  method: string;
  path: string;
  errorCount: number;
  totalCount: number;
  errorRate: number;
}

export interface ErrorReport {
  statusDistribution: ErrorStatusEntry[];
  topErrorEndpoints: ErrorEndpoint[];
}

export interface EndpointStat {
  method: string;
  path: string;
  count: number;
  avgDuration: number;
  p95Duration: number;
  errorRate: number;
  avgResponseSize: number;
}

export type EndpointSortKey = "count" | "avgDuration" | "errorRate";

export interface DuplicateRequestGroup {
  method: string;
  url: string;
  count: number;
  windowMs: number;
  firstSeen: number;
  lastSeen: number;
}

export interface NPlusOnePattern {
  baseUrl: string;
  pattern: string;
  count: number;
  suggestion: string;
}

export interface CorsIssue {
  url: string;
  method: string;
  issue: string;
  timestamp: number;
}

export interface MixedContentWarning {
  url: string;
  referrer: string;
  timestamp: number;
}

export interface PayloadSizeEntry {
  url: string;
  method: string;
  size: number;
  direction: "request" | "response";
}

export interface MiscReport {
  corsIssues: CorsIssue[];
  mixedContentWarnings: MixedContentWarning[];
  largestPayloads: PayloadSizeEntry[];
}

// ── Helper functions ──

function getDuration(tx: HttpTransaction): number | null {
  if (!tx.request || !tx.response) return null;
  return tx.response.time - tx.request.time;
}

function getPath(uri: string): string {
  try {
    const url = new URL(uri);
    return url.pathname;
  } catch {
    return uri;
  }
}

function percentile(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0;
  const idx = Math.ceil((p / 100) * sorted.length) - 1;
  return sorted[Math.max(0, idx)];
}

function isError(status: number): boolean {
  return status >= 400;
}

// ── Analysis Functions ──

export function computeSummary(transactions: HttpTransaction[]): SummaryMetrics {
  const durations: number[] = [];
  let errorCount = 0;

  for (const tx of transactions) {
    const d = getDuration(tx);
    if (d != null && d >= 0) durations.push(d);
    if (tx.response && isError(tx.response.status)) errorCount += 1;
  }

  durations.sort((a, b) => a - b);

  return {
    totalRequests: transactions.length,
    errorRate: transactions.length > 0 ? (errorCount / transactions.length) * 100 : 0,
    avgResponseTime:
      durations.length > 0 ? durations.reduce((a, b) => a + b, 0) / durations.length : 0,
    p95ResponseTime: percentile(durations, 95),
  };
}

export function analyzePerformance(transactions: HttpTransaction[], limit = 20): PerformanceReport {
  const items: { tx: HttpTransaction; duration: number }[] = [];

  for (const tx of transactions) {
    const d = getDuration(tx);
    if (d != null && d >= 0) items.push({ tx, duration: d });
  }

  items.sort((a, b) => b.duration - a.duration);

  const durations = items.map((i) => i.duration).sort((a, b) => a - b);

  const slowRequests: SlowRequest[] = items.slice(0, limit).map((i) => ({
    url: i.tx.request?.uri ?? "",
    method: i.tx.request?.method ?? "",
    durationMs: i.duration,
    timestamp: i.tx.request?.time ?? 0,
    status: i.tx.response?.status ?? null,
  }));

  return {
    slowRequests,
    p50: percentile(durations, 50),
    p95: percentile(durations, 95),
    p99: percentile(durations, 99),
  };
}

export function analyzeErrors(transactions: HttpTransaction[], topLimit = 10): ErrorReport {
  const statusCounts = new Map<number, number>();
  const endpointMap = new Map<
    string,
    { total: number; errors: number; method: string; path: string }
  >();

  for (const tx of transactions) {
    if (!tx.request) continue;
    const key = `${tx.request.method} ${getPath(tx.request.uri)}`;

    if (!endpointMap.has(key)) {
      endpointMap.set(key, {
        total: 0,
        errors: 0,
        method: tx.request.method,
        path: getPath(tx.request.uri),
      });
    }
    const ep = endpointMap.get(key)!;
    ep.total += 1;

    if (tx.response && isError(tx.response.status)) {
      ep.errors += 1;
      const status = tx.response.status;
      statusCounts.set(status, (statusCounts.get(status) ?? 0) + 1);
    }
  }

  const statusDistribution: ErrorStatusEntry[] = Array.from(statusCounts.entries())
    .map(([statusCode, count]) => ({ statusCode, count }))
    .sort((a, b) => b.count - a.count);

  const topErrorEndpoints: ErrorEndpoint[] = Array.from(endpointMap.values())
    .filter((ep) => ep.errors > 0)
    .map((ep) => ({
      method: ep.method,
      path: ep.path,
      errorCount: ep.errors,
      totalCount: ep.total,
      errorRate: (ep.errors / ep.total) * 100,
    }))
    .sort((a, b) => b.errorRate - a.errorRate)
    .slice(0, topLimit);

  return { statusDistribution, topErrorEndpoints };
}

export function analyzeEndpoints(
  transactions: HttpTransaction[],
  sortBy: EndpointSortKey = "count",
): EndpointStat[] {
  const map = new Map<
    string,
    {
      method: string;
      path: string;
      durations: number[];
      errorCount: number;
      responseSizes: number[];
    }
  >();

  for (const tx of transactions) {
    if (!tx.request) continue;
    const key = `${tx.request.method} ${getPath(tx.request.uri)}`;

    if (!map.has(key)) {
      map.set(key, {
        method: tx.request.method,
        path: getPath(tx.request.uri),
        durations: [],
        errorCount: 0,
        responseSizes: [],
      });
    }
    const ep = map.get(key)!;

    const d = getDuration(tx);
    if (d != null && d >= 0) ep.durations.push(d);
    if (tx.response) {
      if (isError(tx.response.status)) ep.errorCount += 1;
      ep.responseSizes.push(tx.response.body_size ?? 0);
    }
  }

  const stats: EndpointStat[] = Array.from(map.values()).map((ep) => {
    const sorted = [...ep.durations].sort((a, b) => a - b);
    const count =
      ep.durations.length + (ep.durations.length === 0 ? ep.responseSizes.length || 1 : 0);
    const totalCount = Math.max(ep.durations.length, ep.responseSizes.length, 1);
    return {
      method: ep.method,
      path: ep.path,
      count: totalCount,
      avgDuration: sorted.length > 0 ? sorted.reduce((a, b) => a + b, 0) / sorted.length : 0,
      p95Duration: percentile(sorted, 95),
      errorRate: totalCount > 0 ? (ep.errorCount / totalCount) * 100 : 0,
      avgResponseSize:
        ep.responseSizes.length > 0
          ? ep.responseSizes.reduce((a, b) => a + b, 0) / ep.responseSizes.length
          : 0,
    };
  });

  const sorters: Record<EndpointSortKey, (a: EndpointStat, b: EndpointStat) => number> = {
    count: (a, b) => b.count - a.count,
    avgDuration: (a, b) => b.avgDuration - a.avgDuration,
    errorRate: (a, b) => b.errorRate - a.errorRate,
  };

  stats.sort(sorters[sortBy]);
  return stats;
}

export function detectDuplicateRequests(
  transactions: HttpTransaction[],
  windowMs = 1000,
): DuplicateRequestGroup[] {
  // Group by method + url
  const groups = new Map<string, { method: string; url: string; timestamps: number[] }>();

  for (const tx of transactions) {
    if (!tx.request) continue;
    const key = `${tx.request.method} ${tx.request.uri}`;
    if (!groups.has(key)) {
      groups.set(key, {
        method: tx.request.method,
        url: tx.request.uri,
        timestamps: [],
      });
    }
    groups.get(key)!.timestamps.push(tx.request.time);
  }

  const results: DuplicateRequestGroup[] = [];

  for (const group of groups.values()) {
    if (group.timestamps.length < 2) continue;
    group.timestamps.sort((a, b) => a - b);

    // Find clusters within window
    let clusterStart = 0;
    for (let i = 1; i <= group.timestamps.length; i += 1) {
      if (
        i === group.timestamps.length ||
        group.timestamps[i] - group.timestamps[i - 1] > windowMs
      ) {
        const clusterSize = i - clusterStart;
        if (clusterSize >= 2) {
          results.push({
            method: group.method,
            url: group.url,
            count: clusterSize,
            windowMs,
            firstSeen: group.timestamps[clusterStart],
            lastSeen: group.timestamps[i - 1],
          });
        }
        clusterStart = i;
      }
    }
  }

  results.sort((a, b) => b.count - a.count);
  return results;
}

export function detectNPlusOne(transactions: HttpTransaction[], threshold = 3): NPlusOnePattern[] {
  // Group sequential requests by normalized URL pattern
  const patternCounts = new Map<string, { baseUrl: string; pattern: string; count: number }>();

  // Sort by time
  const sorted = [...transactions]
    .filter((tx) => tx.request)
    .sort((a, b) => (a.request?.time ?? 0) - (b.request?.time ?? 0));

  // Detect repetitive patterns: same base path with different IDs
  const pathPattern = /^(.*\/)[^/]+$/;

  for (const tx of sorted) {
    if (!tx.request) continue;
    const path = getPath(tx.request.uri);
    const match = pathPattern.exec(path);
    if (!match) continue;

    const basePath = match[1];
    const key = `${tx.request.method} ${basePath}`;

    if (!patternCounts.has(key)) {
      patternCounts.set(key, {
        baseUrl: basePath,
        pattern: `${tx.request.method} ${basePath}*`,
        count: 0,
      });
    }
    patternCounts.get(key)!.count += 1;
  }

  return Array.from(patternCounts.values())
    .filter((p) => p.count >= threshold)
    .map((p) => ({
      baseUrl: p.baseUrl,
      pattern: p.pattern,
      count: p.count,
      suggestion: `${p.count}개의 개별 요청을 배치 API로 통합하는 것을 고려하세요.`,
    }))
    .sort((a, b) => b.count - a.count);
}

export function analyzeMisc(transactions: HttpTransaction[]): MiscReport {
  const corsIssues: CorsIssue[] = [];
  const mixedContentWarnings: MixedContentWarning[] = [];
  const payloads: PayloadSizeEntry[] = [];

  for (const tx of transactions) {
    if (!tx.request) continue;

    // CORS detection
    const origin = tx.request.headers?.origin;
    if (origin && tx.response) {
      const allowOrigin = tx.response.headers?.["access-control-allow-origin"];
      if (!allowOrigin) {
        corsIssues.push({
          url: tx.request.uri,
          method: tx.request.method,
          issue: "Access-Control-Allow-Origin 헤더 누락",
          timestamp: tx.request.time,
        });
      } else if (allowOrigin !== "*" && allowOrigin !== origin) {
        corsIssues.push({
          url: tx.request.uri,
          method: tx.request.method,
          issue: `Origin 불일치: ${origin} != ${allowOrigin}`,
          timestamp: tx.request.time,
        });
      }
    }

    // Mixed content detection
    if (tx.request.uri.startsWith("http://")) {
      const referer = tx.request.headers?.referer ?? tx.request.headers?.Referer;
      if (referer && referer.startsWith("https://")) {
        mixedContentWarnings.push({
          url: tx.request.uri,
          referrer: referer,
          timestamp: tx.request.time,
        });
      }
    }

    // Payload sizes
    if (tx.request.body_size > 0) {
      payloads.push({
        url: tx.request.uri,
        method: tx.request.method,
        size: tx.request.body_size,
        direction: "request",
      });
    }
    if (tx.response && tx.response.body_size > 0) {
      payloads.push({
        url: tx.request.uri,
        method: tx.request.method,
        size: tx.response.body_size,
        direction: "response",
      });
    }
  }

  payloads.sort((a, b) => b.size - a.size);

  return {
    corsIssues,
    mixedContentWarnings,
    largestPayloads: payloads.slice(0, 10),
  };
}
