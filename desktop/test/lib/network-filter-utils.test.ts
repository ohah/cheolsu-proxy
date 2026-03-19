import { describe, test, expect } from "bun:test";

import type { HttpTransaction } from "../../src/entities/proxy";
import { getFilteredTransactions } from "../../src/pages/network-dashboard/lib/utils";

function createTx(
  id: string,
  opts: {
    method?: string;
    uri?: string;
    status?: number;
    clientAddr?: string;
    proxyAuthUser?: string;
  } = {},
): HttpTransaction {
  return {
    request: {
      id,
      method: opts.method ?? "GET",
      uri: opts.uri ?? `http://example.com/api/${id}`,
      version: "HTTP/1.1",
      headers: {},
      body: null,
      time: 1000,
      data_type: "Text",
      body_size: 0,
      client_addr: opts.clientAddr,
      proxy_auth_user: opts.proxyAuthUser,
    },
    response:
      opts.status !== undefined
        ? {
            id,
            status: opts.status,
            version: "HTTP/1.1",
            headers: {},
            body: null,
            time: 1100,
            data_type: "Text",
            body_size: 0,
          }
        : null,
  } as HttpTransaction;
}

describe("getFilteredTransactions", () => {
  const transactions = [
    createTx("1", { method: "GET", uri: "http://example.com/api/users", status: 200 }),
    createTx("2", { method: "POST", uri: "http://example.com/api/users", status: 201 }),
    createTx("3", { method: "GET", uri: "http://example.com/api/posts", status: 404 }),
    createTx("4", { method: "DELETE", uri: "http://example.com/api/posts/1", status: 500 }),
    createTx("5", { method: "PUT", uri: "https://other.com/data", status: 200 }),
  ];

  test("필터 없으면 전체 반환", () => {
    const result = getFilteredTransactions(transactions, [], [], []);
    expect(result).toHaveLength(5);
  });

  // --- 상태 필터 ---
  test("상태 코드 필터링 (정확한 값)", () => {
    const result = getFilteredTransactions(transactions, ["200"], [], []);
    expect(result).toHaveLength(2);
    expect(result.every((t) => t.response?.status === 200)).toBe(true);
  });

  test("상태 코드 범위 필터링 (2xx)", () => {
    const result = getFilteredTransactions(transactions, ["2xx"], [], []);
    expect(result).toHaveLength(3); // 200, 201, 200
  });

  test("상태 코드 범위 필터링 (4xx)", () => {
    const result = getFilteredTransactions(transactions, ["4xx"], [], []);
    expect(result).toHaveLength(1);
    expect(result[0].response?.status).toBe(404);
  });

  test("상태 코드 제외 필터링", () => {
    const result = getFilteredTransactions(transactions, [], [], [], ["2xx"]);
    expect(result).toHaveLength(2); // 404, 500
  });

  test("상태 코드 제외가 포함보다 우선", () => {
    const result = getFilteredTransactions(transactions, ["200"], [], [], ["2xx"]);
    expect(result).toHaveLength(0);
  });

  // --- 메서드 필터 ---
  test("메서드 필터링", () => {
    const result = getFilteredTransactions(transactions, [], ["GET"], []);
    expect(result).toHaveLength(2);
  });

  test("메서드 제외 필터링", () => {
    const result = getFilteredTransactions(transactions, [], [], [], [], ["GET"]);
    expect(result).toHaveLength(3);
  });

  test("메서드 필터 대소문자 무시", () => {
    const txLower = [createTx("1", { method: "get", status: 200 })];
    const result = getFilteredTransactions(txLower, [], ["GET"], []);
    expect(result).toHaveLength(1);
  });

  // --- 경로 필터 ---
  test("경로 포함 필터링", () => {
    const result = getFilteredTransactions(transactions, [], [], ["users"]);
    expect(result).toHaveLength(2);
  });

  test("경로 와일드카드 필터링", () => {
    const result = getFilteredTransactions(transactions, [], [], ["*/posts*"]);
    expect(result).toHaveLength(2);
  });

  test("경로 대소문자 무시", () => {
    const result = getFilteredTransactions(transactions, [], [], ["USERS"]);
    expect(result).toHaveLength(2);
  });

  test("경로 제외 필터링", () => {
    const result = getFilteredTransactions(transactions, [], [], [], [], [], ["posts"]);
    expect(result).toHaveLength(3); // users(2) + other.com(1)
  });

  test("경로 필터 AND 조건", () => {
    const result = getFilteredTransactions(transactions, [], [], ["example.com", "users"]);
    expect(result).toHaveLength(2);
  });

  // --- 클라이언트 필터 ---
  test("클라이언트 IP 필터링", () => {
    const txsWithClient = [
      createTx("1", { clientAddr: "192.168.1.1:54321", status: 200 }),
      createTx("2", { clientAddr: "[::1]:54321", status: 200 }),
      createTx("3", { clientAddr: "10.0.0.1:12345", status: 200 }),
    ];
    const result = getFilteredTransactions(txsWithClient, [], [], [], [], [], [], ["192.168"], []);
    expect(result).toHaveLength(1);
  });

  test("IPv6 대괄호 제거 후 매칭", () => {
    const txsIPv6 = [createTx("1", { clientAddr: "[::1]:54321", status: 200 })];
    const result = getFilteredTransactions(txsIPv6, [], [], [], [], [], [], ["::1"], []);
    expect(result).toHaveLength(1);
  });

  test("클라이언트 태그로 필터링", () => {
    const txsWithClient = [
      createTx("1", { clientAddr: "192.168.1.1:54321", status: 200 }),
      createTx("2", { clientAddr: "10.0.0.1:12345", status: 200 }),
    ];
    const tags = { "192.168.1.1": "mobile-device" };
    const result = getFilteredTransactions(
      txsWithClient,
      [],
      [],
      [],
      [],
      [],
      [],
      ["mobile"],
      [],
      tags,
    );
    expect(result).toHaveLength(1);
  });

  test("proxy auth user로 필터링", () => {
    const txsWithAuth = [
      createTx("1", { proxyAuthUser: "admin", status: 200 }),
      createTx("2", { proxyAuthUser: "guest", status: 200 }),
    ];
    const result = getFilteredTransactions(txsWithAuth, [], [], [], [], [], [], ["admin"], []);
    expect(result).toHaveLength(1);
  });

  // --- OR 연산자 ---
  test("OR 연산자: 포함 필터 중 하나라도 매칭되면 통과", () => {
    const result = getFilteredTransactions(
      transactions,
      ["404"],
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
    // status=404 또는 method=POST → tx3(404/GET) + tx2(201/POST)
    expect(result).toHaveLength(2);
  });

  test("OR 연산자: 포함 필터 없이 제외만 있으면 AND처럼 동작", () => {
    const result = getFilteredTransactions(
      transactions,
      [],
      [],
      [],
      ["5xx"],
      ["DELETE"],
      [],
      [],
      [],
      {},
      "or",
    );
    // 제외: status 5xx OR method DELETE → tx4(500/DELETE) 제외, tx3(404/GET)은 통과
    expect(result).toHaveLength(4);
  });

  // --- 복합 ---
  test("복합 필터: 메서드 + 상태 + 경로", () => {
    const result = getFilteredTransactions(transactions, ["2xx"], ["GET"], ["example.com"]);
    expect(result).toHaveLength(1);
    expect(result[0].request?.uri).toContain("users");
  });
});
