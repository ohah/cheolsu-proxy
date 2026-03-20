import { describe, test, expect } from "bun:test";

import type { HttpTransaction } from "../../src/entities/proxy";
import {
  computeSummary,
  analyzePerformance,
  analyzeErrors,
  analyzeEndpoints,
  detectDuplicateRequests,
  detectNPlusOne,
  analyzeMisc,
} from "../../src/pages/analytics-dashboard/lib/analytics-engine";
import { NANOS_PER_MS as MS } from "../../src/shared/lib/format-time";

function createTx(
  id: string,
  opts: {
    method?: string;
    uri?: string;
    status?: number;
    reqTime?: number;
    resTime?: number;
    reqBodySize?: number;
    resBodySize?: number;
    headers?: Record<string, string>;
    resHeaders?: Record<string, string>;
  } = {},
): HttpTransaction {
  const reqTime = opts.reqTime ?? 1000 * MS;
  const resTime = opts.resTime ?? reqTime + 100 * MS;
  return {
    request: {
      id,
      method: opts.method ?? "GET",
      uri: opts.uri ?? `http://example.com/api/${id}`,
      version: "HTTP/1.1",
      headers: opts.headers ?? {},
      body: null,
      time: reqTime,
      data_type: "text",
      body_size: opts.reqBodySize ?? 0,
    },
    response:
      opts.status !== undefined
        ? {
            id,
            status: opts.status,
            version: "HTTP/1.1",
            headers: opts.resHeaders ?? {},
            body: null,
            time: resTime,
            data_type: "text",
            body_size: opts.resBodySize ?? 0,
          }
        : null,
  } as HttpTransaction;
}

describe("computeSummary", () => {
  test("returns zero metrics for empty transactions", () => {
    const result = computeSummary([]);
    expect(result.totalRequests).toBe(0);
    expect(result.errorRate).toBe(0);
    expect(result.avgResponseTime).toBe(0);
    expect(result.p95ResponseTime).toBe(0);
  });

  test("computes correct metrics", () => {
    const txs = [
      createTx("1", { reqTime: 0, resTime: 100 * MS, status: 200 }),
      createTx("2", { reqTime: 0, resTime: 200 * MS, status: 200 }),
      createTx("3", { reqTime: 0, resTime: 300 * MS, status: 500 }),
    ];
    const result = computeSummary(txs);
    expect(result.totalRequests).toBe(3);
    expect(result.errorRate).toBeCloseTo(33.33, 1);
    expect(result.avgResponseTime).toBe(200);
  });

  test("handles transactions without response", () => {
    const txs = [createTx("1")];
    const result = computeSummary(txs);
    expect(result.totalRequests).toBe(1);
    expect(result.errorRate).toBe(0);
  });
});

describe("analyzePerformance", () => {
  test("returns slow requests sorted by duration", () => {
    const txs = [
      createTx("1", { reqTime: 0, resTime: 50 * MS, status: 200 }),
      createTx("2", { reqTime: 0, resTime: 500 * MS, status: 200 }),
      createTx("3", { reqTime: 0, resTime: 200 * MS, status: 200 }),
    ];
    const result = analyzePerformance(txs, 2);
    expect(result.slowRequests.length).toBe(2);
    expect(result.slowRequests[0].durationMs).toBe(500);
    expect(result.slowRequests[1].durationMs).toBe(200);
  });

  test("computes percentiles correctly", () => {
    const txs = Array.from({ length: 100 }, (_, i) =>
      createTx(String(i), { reqTime: 0, resTime: (i + 1) * MS, status: 200 }),
    );
    const result = analyzePerformance(txs);
    expect(result.p50).toBe(50);
    expect(result.p95).toBe(95);
    expect(result.p99).toBe(99);
  });
});

describe("analyzeErrors", () => {
  test("counts error status codes", () => {
    const txs = [
      createTx("1", { status: 404 }),
      createTx("2", { status: 404 }),
      createTx("3", { status: 500 }),
      createTx("4", { status: 200 }),
    ];
    const result = analyzeErrors(txs);
    expect(result.statusDistribution.length).toBe(2);
    const s404 = result.statusDistribution.find((s) => s.statusCode === 404);
    expect(s404?.count).toBe(2);
  });

  test("returns empty when no errors", () => {
    const txs = [createTx("1", { status: 200 })];
    const result = analyzeErrors(txs);
    expect(result.statusDistribution.length).toBe(0);
    expect(result.topErrorEndpoints.length).toBe(0);
  });
});

describe("analyzeEndpoints", () => {
  test("groups by method + path", () => {
    const txs = [
      createTx("1", {
        method: "GET",
        uri: "http://a.com/api/users",
        reqTime: 0,
        resTime: 100 * MS,
        status: 200,
      }),
      createTx("2", {
        method: "GET",
        uri: "http://a.com/api/users",
        reqTime: 0,
        resTime: 200 * MS,
        status: 200,
      }),
      createTx("3", {
        method: "POST",
        uri: "http://a.com/api/users",
        reqTime: 0,
        resTime: 50 * MS,
        status: 201,
      }),
    ];
    const result = analyzeEndpoints(txs, "count");
    expect(result.length).toBe(2);
    const getEndpoint = result.find((e) => e.method === "GET");
    expect(getEndpoint?.count).toBe(2);
    expect(getEndpoint?.avgDuration).toBe(150);
  });

  test("sorts by different keys", () => {
    const txs = [
      createTx("1", {
        method: "GET",
        uri: "http://a.com/a",
        reqTime: 0,
        resTime: 500 * MS,
        status: 200,
      }),
      createTx("2", {
        method: "GET",
        uri: "http://a.com/b",
        reqTime: 0,
        resTime: 100 * MS,
        status: 500,
      }),
    ];
    const byDuration = analyzeEndpoints(txs, "avgDuration");
    expect(byDuration[0].path).toBe("/a");
    const byError = analyzeEndpoints(txs, "errorRate");
    expect(byError[0].path).toBe("/b");
  });
});

describe("detectDuplicateRequests", () => {
  test("detects duplicates within window", () => {
    const txs = [
      createTx("1", { method: "GET", uri: "http://a.com/api", reqTime: 1000 * MS }),
      createTx("2", { method: "GET", uri: "http://a.com/api", reqTime: 1100 * MS }),
      createTx("3", { method: "GET", uri: "http://a.com/api", reqTime: 1200 * MS }),
    ];
    const result = detectDuplicateRequests(txs, 500);
    expect(result.length).toBe(1);
    expect(result[0].count).toBe(3);
  });

  test("does not detect non-duplicates", () => {
    const txs = [
      createTx("1", { method: "GET", uri: "http://a.com/api", reqTime: 1000 * MS }),
      createTx("2", { method: "GET", uri: "http://a.com/api", reqTime: 5000 * MS }),
    ];
    const result = detectDuplicateRequests(txs, 500);
    expect(result.length).toBe(0);
  });
});

describe("detectNPlusOne", () => {
  test("detects N+1 patterns", () => {
    const txs = [
      createTx("1", { method: "GET", uri: "http://a.com/api/users/1", reqTime: 100 * MS }),
      createTx("2", { method: "GET", uri: "http://a.com/api/users/2", reqTime: 200 * MS }),
      createTx("3", { method: "GET", uri: "http://a.com/api/users/3", reqTime: 300 * MS }),
      createTx("4", { method: "GET", uri: "http://a.com/api/users/4", reqTime: 400 * MS }),
    ];
    const result = detectNPlusOne(txs, 3);
    expect(result.length).toBe(1);
    expect(result[0].count).toBe(4);
    expect(result[0].pattern).toContain("/api/users/");
  });

  test("does not detect when below threshold", () => {
    const txs = [
      createTx("1", { method: "GET", uri: "http://a.com/api/users/1", reqTime: 100 * MS }),
      createTx("2", { method: "GET", uri: "http://a.com/api/users/2", reqTime: 200 * MS }),
    ];
    const result = detectNPlusOne(txs, 3);
    expect(result.length).toBe(0);
  });
});

describe("analyzeMisc", () => {
  test("detects CORS issues", () => {
    const txs = [
      createTx("1", {
        uri: "http://api.com/data",
        status: 200,
        headers: { origin: "http://app.com" },
        resHeaders: {},
      }),
    ];
    const result = analyzeMisc(txs);
    expect(result.corsIssues.length).toBe(1);
    expect(result.corsIssues[0].issue).toContain("Access-Control-Allow-Origin");
  });

  test("detects mixed content", () => {
    const txs = [
      createTx("1", {
        uri: "http://insecure.com/img.png",
        status: 200,
        headers: { referer: "https://secure.com/page" },
      }),
    ];
    const result = analyzeMisc(txs);
    expect(result.mixedContentWarnings.length).toBe(1);
  });

  test("returns largest payloads sorted by size", () => {
    const txs = [
      createTx("1", { uri: "http://a.com/small", reqBodySize: 100, resBodySize: 200, status: 200 }),
      createTx("2", {
        uri: "http://a.com/big",
        reqBodySize: 5000,
        resBodySize: 10000,
        status: 200,
      }),
    ];
    const result = analyzeMisc(txs);
    expect(result.largestPayloads[0].size).toBe(10000);
  });
});
