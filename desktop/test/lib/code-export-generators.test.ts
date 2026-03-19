import { describe, test, expect } from "bun:test";

import type { HttpTransaction } from "../../src/entities/proxy";
import { generateCurlCommand } from "../../src/shared/lib/curl";
import { generateFetchCommand } from "../../src/shared/lib/fetch";

function createTx(
  opts: {
    method?: string;
    uri?: string;
    headers?: Record<string, string>;
    body?: number[];
    dataType?: string;
  } = {},
): HttpTransaction {
  return {
    request: {
      id: "1",
      method: opts.method ?? "GET",
      uri: opts.uri ?? "http://example.com/api/users",
      version: "HTTP/1.1",
      headers: opts.headers ?? {},
      body: opts.body ? new Uint8Array(opts.body) : null,
      time: 1000,
      data_type: opts.dataType ?? "Text",
      body_size: opts.body?.length ?? 0,
    },
    response: null,
  } as HttpTransaction;
}

describe("generateCurlCommand", () => {
  test("request 없으면 기본 curl 반환", () => {
    const tx = { request: null, response: null } as unknown as HttpTransaction;
    expect(generateCurlCommand(tx)).toBe("curl -X GET 'http://localhost'");
  });

  test("기본 GET 요청", () => {
    const tx = createTx();
    const result = generateCurlCommand(tx);
    expect(result).toContain("curl -X GET");
    expect(result).toContain("http://example.com/api/users");
  });

  test("헤더 포함", () => {
    const tx = createTx({
      headers: { "Content-Type": "application/json", Accept: "application/json" },
    });
    const result = generateCurlCommand(tx);
    expect(result).toContain("-H 'Content-Type: application/json'");
    expect(result).toContain("-H 'Accept: application/json'");
  });

  test("POST 요청 + JSON 바디", () => {
    const body = Array.from(new TextEncoder().encode('{"name":"test"}'));
    const tx = createTx({
      method: "POST",
      body,
      dataType: "Json",
      headers: { "Content-Type": "application/json" },
    });
    const result = generateCurlCommand(tx);
    expect(result).toContain("curl -X POST");
    expect(result).toContain("-d '");
    expect(result).toContain('"name":"test"');
  });

  test("작은따옴표 이스케이프", () => {
    const tx = createTx({ uri: "http://example.com/api?name=O'Brien" });
    const result = generateCurlCommand(tx);
    expect(result).toContain("O'\\''Brien");
  });

  test("바이너리 데이터는 -d 없음", () => {
    const tx = createTx({ body: [0x89, 0x50], dataType: "Image" });
    const result = generateCurlCommand(tx);
    expect(result).not.toContain("-d");
  });
});

describe("generateFetchCommand", () => {
  test("request 없으면 기본 fetch 반환", () => {
    const tx = { request: null, response: null } as unknown as HttpTransaction;
    expect(generateFetchCommand(tx)).toBe('fetch("http://localhost")');
  });

  test("기본 GET 요청 (옵션 없음)", () => {
    const tx = createTx();
    const result = generateFetchCommand(tx);
    expect(result).toBe('fetch("http://example.com/api/users")');
  });

  test("POST 요청은 method 포함", () => {
    const tx = createTx({ method: "POST" });
    const result = generateFetchCommand(tx);
    expect(result).toContain('method: "POST"');
  });

  test("헤더 포함", () => {
    const tx = createTx({ headers: { "Content-Type": "application/json" } });
    const result = generateFetchCommand(tx);
    expect(result).toContain("headers:");
    expect(result).toContain('"Content-Type": "application/json"');
  });

  test("JSON 바디는 JSON.stringify로 감싸기", () => {
    const body = Array.from(new TextEncoder().encode('{"name":"test"}'));
    const tx = createTx({
      method: "POST",
      body,
      dataType: "Json",
    });
    const result = generateFetchCommand(tx);
    expect(result).toContain("body: JSON.stringify(");
  });

  test("텍스트 바디는 문자열로", () => {
    const body = Array.from(new TextEncoder().encode("plain text body"));
    const tx = createTx({
      method: "POST",
      body,
      dataType: "Text",
    });
    const result = generateFetchCommand(tx);
    expect(result).toContain("body:");
    expect(result).toContain("plain text body");
  });

  test("쌍따옴표 이스케이프", () => {
    const tx = createTx({ uri: 'http://example.com/api?q="test"' });
    const result = generateFetchCommand(tx);
    expect(result).toContain('\\"test\\"');
  });

  test("바이너리 데이터는 body 없음", () => {
    const tx = createTx({ method: "POST", body: [0x89, 0x50], dataType: "Image" });
    const result = generateFetchCommand(tx);
    expect(result).not.toContain("body:");
  });
});
