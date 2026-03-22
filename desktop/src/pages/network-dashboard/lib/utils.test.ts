import { describe, test, expect } from "bun:test";
import type { HttpTransaction } from "@/entities/proxy";
import { getFilteredTransactions } from "./utils";

function createTransaction(
  overrides: {
    method?: string;
    uri?: string;
    status?: number;
    clientAddr?: string;
    proxyAuthUser?: string;
  } = {},
): HttpTransaction {
  return {
    request: {
      method: overrides.method ?? "GET",
      uri: overrides.uri ?? "https://example.com/api",
      version: "HTTP/1.1",
      headers: {},
      body: null,
      time: Date.now() * 1_000_000,
      id: crypto.randomUUID(),
      data_type: "Unknown",
      body_size: 0,
      client_addr: overrides.clientAddr,
      proxy_auth_user: overrides.proxyAuthUser,
    },
    response:
      overrides.status != null
        ? {
            status: overrides.status,
            version: "HTTP/1.1",
            headers: {},
            body: null,
            time: Date.now() * 1_000_000,
            id: "",
            data_type: "Unknown",
            body_size: 0,
          }
        : null,
  };
}

describe("getFilteredTransactions", () => {
  const txs: HttpTransaction[] = [
    createTransaction({ method: "GET", uri: "https://api.example.com/users", status: 200 }),
    createTransaction({ method: "POST", uri: "https://api.example.com/users", status: 201 }),
    createTransaction({ method: "GET", uri: "https://cdn.example.com/image.png", status: 304 }),
    createTransaction({ method: "OPTIONS", uri: "https://api.example.com/users", status: 204 }),
    createTransaction({ method: "GET", uri: "https://api.example.com/health", status: 500 }),
  ];

  test("필터가 없으면 모든 트랜잭션을 반환한다", () => {
    const result = getFilteredTransactions(txs, [], [], []);
    expect(result).toHaveLength(5);
  });

  // 메서드 필터
  test("method 포함 필터로 GET만 필터링한다", () => {
    const result = getFilteredTransactions(txs, [], ["GET"], []);
    expect(result).toHaveLength(3);
    expect(result.every((t) => t.request?.method === "GET")).toBe(true);
  });

  test("method 제외 필터로 OPTIONS를 제외한다", () => {
    const result = getFilteredTransactions(txs, [], [], [], [], ["OPTIONS"]);
    expect(result).toHaveLength(4);
    expect(result.some((t) => t.request?.method === "OPTIONS")).toBe(false);
  });

  // 상태 코드 필터
  test("status 필터로 2xx만 필터링한다", () => {
    const result = getFilteredTransactions(txs, ["2xx"], [], []);
    expect(result).toHaveLength(3);
  });

  test("status 필터로 정확한 코드 필터링한다", () => {
    const result = getFilteredTransactions(txs, ["200"], [], []);
    expect(result).toHaveLength(1);
  });

  test("status 제외 필터로 5xx를 제외한다", () => {
    const result = getFilteredTransactions(txs, [], [], [], ["5xx"]);
    expect(result).toHaveLength(4);
  });

  // URL 필터
  test("URL 포함 필터로 api 경로만 필터링한다", () => {
    const result = getFilteredTransactions(txs, [], [], ["api.example.com"]);
    expect(result).toHaveLength(4);
  });

  test("URL 제외 필터로 cdn 경로를 제외한다", () => {
    const result = getFilteredTransactions(txs, [], [], [], [], [], ["cdn.example"]);
    expect(result).toHaveLength(4);
  });

  test("URL 와일드카드 필터를 지원한다", () => {
    const result = getFilteredTransactions(txs, [], [], ["*/users"]);
    expect(result).toHaveLength(3);
  });

  // 클라이언트 필터
  test("client 필터로 IP 주소를 필터링한다", () => {
    const clientTxs = [
      createTransaction({ clientAddr: "192.168.1.1:50000" }),
      createTransaction({ clientAddr: "10.0.0.1:50001" }),
      createTransaction({ clientAddr: "192.168.1.2:50002" }),
    ];
    const result = getFilteredTransactions(clientTxs, [], [], [], [], [], [], ["192.168"], [], {});
    expect(result).toHaveLength(2);
  });

  test("client 태그로 필터링한다", () => {
    const clientTxs = [
      createTransaction({ clientAddr: "192.168.1.1:50000" }),
      createTransaction({ clientAddr: "10.0.0.1:50001" }),
    ];
    const tags = { "192.168.1.1": "MyPhone" };
    const result = getFilteredTransactions(
      clientTxs,
      [],
      [],
      [],
      [],
      [],
      [],
      ["MyPhone"],
      [],
      tags,
    );
    expect(result).toHaveLength(1);
  });

  test("client 제외 필터로 특정 IP를 제외한다", () => {
    const clientTxs = [
      createTransaction({ clientAddr: "192.168.1.1:50000" }),
      createTransaction({ clientAddr: "10.0.0.1:50001" }),
    ];
    const result = getFilteredTransactions(clientTxs, [], [], [], [], [], [], [], ["10.0.0"], {});
    expect(result).toHaveLength(1);
  });

  // OR 연산자
  test("OR 연산자로 필터를 결합한다", () => {
    const result = getFilteredTransactions(
      txs,
      ["5xx"],
      ["POST"],
      [],
      [],
      [],
      [],
      [],
      [],
      {},
      "or",
    );
    // POST(1개) + 5xx(1개) = 2개
    expect(result).toHaveLength(2);
  });

  // AND 연산자 (기본값)
  test("AND 연산자로 필터를 결합한다", () => {
    const result = getFilteredTransactions(
      txs,
      ["2xx"],
      ["GET"],
      [],
      [],
      [],
      [],
      [],
      [],
      {},
      "and",
    );
    // GET이면서 2xx인 것만: 200, 304 → 2개 (304는 3xx이므로 제외) → 1개
    expect(result).toHaveLength(1);
  });

  // proxy auth user 필터
  test("proxy auth user로 필터링한다", () => {
    const clientTxs = [
      createTransaction({ clientAddr: "10.0.0.1:50000", proxyAuthUser: "admin" }),
      createTransaction({ clientAddr: "10.0.0.2:50001", proxyAuthUser: "guest" }),
    ];
    const result = getFilteredTransactions(clientTxs, [], [], [], [], [], [], ["admin"], [], {});
    expect(result).toHaveLength(1);
  });
});
