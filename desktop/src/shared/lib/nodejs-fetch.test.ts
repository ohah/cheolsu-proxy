import { describe, test, expect } from "bun:test";

import { generateNodejsFetchCommand } from "./nodejs-fetch";
import type { HttpTransaction } from "@/entities/proxy/model/types";
import type { DataType } from "@/entities/proxy/model/data-type";

const createTransaction = (
  overrides: {
    method?: string;
    uri?: string;
    headers?: Record<string, string>;
    body?: string | null;
    data_type?: DataType;
  } = {},
): HttpTransaction => {
  const bodyText = overrides.body ?? null;
  const bodyBytes = bodyText ? new TextEncoder().encode(bodyText) : null;

  return {
    request: {
      method: overrides.method ?? "GET",
      uri: overrides.uri ?? "https://example.com/api",
      version: "HTTP/1.1",
      headers: overrides.headers ?? {},
      body: bodyBytes,
      time: Date.now(),
      id: "test-id",
      data_type: overrides.data_type ?? "Text",
      body_size: bodyBytes ? bodyBytes.length : 0,
    },
    response: null,
  };
};

const createEmptyTransaction = (): HttpTransaction => ({
  request: null,
  response: null,
});

describe("generateNodejsFetchCommand", () => {
  test("request가 없으면 기본 코드 반환", () => {
    const result = generateNodejsFetchCommand(createEmptyTransaction());
    expect(result).toContain('await fetch("http://localhost")');
    expect(result).toContain("await response.text()");
    expect(result).toContain("console.log(data)");
  });

  test("GET 요청 → method 생략", () => {
    const result = generateNodejsFetchCommand(createTransaction());
    expect(result).not.toContain("method:");
    expect(result).toContain('fetch("https://example.com/api"');
  });

  test("POST + JSON body → body: JSON.stringify(...)", () => {
    const result = generateNodejsFetchCommand(
      createTransaction({
        method: "POST",
        body: '{"key":"value"}',
        data_type: "Json",
      }),
    );
    expect(result).toContain('method: "POST"');
    expect(result).toContain("JSON.stringify(");
    expect(result).toContain('{"key":"value"}');
  });

  test("헤더가 포함된 요청", () => {
    const result = generateNodejsFetchCommand(
      createTransaction({
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: "Bearer token123",
        },
      }),
    );
    expect(result).toContain("headers:");
    expect(result).toContain('"Content-Type": "application/json"');
    expect(result).toContain('"Authorization": "Bearer token123"');
  });

  test("POST + 텍스트 바디", () => {
    const result = generateNodejsFetchCommand(
      createTransaction({
        method: "POST",
        body: "plain text body",
        data_type: "Text",
      }),
    );
    expect(result).toContain("body:");
    expect(result).not.toContain("JSON.stringify");
  });

  test("PUT 메서드", () => {
    const result = generateNodejsFetchCommand(createTransaction({ method: "PUT" }));
    expect(result).toContain('method: "PUT"');
  });

  test("DELETE 메서드", () => {
    const result = generateNodejsFetchCommand(createTransaction({ method: "DELETE" }));
    expect(result).toContain('method: "DELETE"');
  });

  test("URL의 특수문자 이스케이프", () => {
    const result = generateNodejsFetchCommand(
      createTransaction({ uri: 'https://example.com/api?q="test"' }),
    );
    expect(result).toContain('\\"test\\"');
  });
});
