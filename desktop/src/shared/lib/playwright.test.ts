import { describe, test, expect } from "bun:test";

import { generatePlaywrightCode } from "./playwright";
import type { HttpTransaction } from "@/entities/proxy/model/types";
import type { DataType } from "@/entities/proxy/model/data-type";

const createTransaction = (
  overrides: {
    method?: string;
    uri?: string;
    headers?: Record<string, string>;
    body?: string | null;
    data_type?: DataType;
    responseStatus?: number;
    responseBodyJson?: unknown;
  } = {},
): HttpTransaction => {
  const bodyText = overrides.body ?? null;
  const bodyBytes = bodyText ? new TextEncoder().encode(bodyText) : null;

  return {
    request: {
      method: overrides.method ?? "GET",
      uri: overrides.uri ?? "https://example.com/api/users",
      version: "HTTP/1.1",
      headers: overrides.headers ?? {},
      body: bodyBytes,
      time: Date.now(),
      id: "test-id",
      data_type: overrides.data_type ?? "Text",
      body_size: bodyBytes ? bodyBytes.length : 0,
    },
    response: {
      status: overrides.responseStatus ?? 200,
      version: "HTTP/1.1",
      headers: {},
      body: null,
      time: Date.now(),
      id: "test-id",
      data_type: "Json",
      body_size: 0,
      body_json: overrides.responseBodyJson ?? undefined,
    },
  };
};

const createEmptyTransaction = (): HttpTransaction => ({
  request: null,
  response: null,
});

describe("generatePlaywrightCode", () => {
  test("request가 없으면 기본 코드 반환", () => {
    const result = generatePlaywrightCode(createEmptyTransaction());
    expect(result).toContain("@playwright/test");
    expect(result).toContain("request.get");
  });

  test("GET 요청 테스트 생성", () => {
    const result = generatePlaywrightCode(createTransaction());
    expect(result).toContain("request.get");
    expect(result).toContain("expect(response.status()).toBe(200)");
  });

  test("POST 요청 테스트 생성", () => {
    const result = generatePlaywrightCode(
      createTransaction({
        method: "POST",
        body: '{"name":"test"}',
        data_type: "Json",
      }),
    );
    expect(result).toContain("request.post");
    expect(result).toContain("data:");
  });

  test("응답 JSON assertion 생성", () => {
    const result = generatePlaywrightCode(
      createTransaction({
        responseBodyJson: { id: 1, name: "test" },
      }),
    );
    expect(result).toContain("await response.json()");
    expect(result).toContain("expect.any(Number)");
    expect(result).toContain("expect.any(String)");
  });

  test("상태 코드 반영", () => {
    const result = generatePlaywrightCode(createTransaction({ responseStatus: 201 }));
    expect(result).toContain("expect(response.status()).toBe(201)");
    expect(result).toContain("should return 201");
  });

  test("PUT 메서드", () => {
    const result = generatePlaywrightCode(createTransaction({ method: "PUT" }));
    expect(result).toContain("request.put");
  });

  test("DELETE 메서드", () => {
    const result = generatePlaywrightCode(createTransaction({ method: "DELETE" }));
    expect(result).toContain("request.delete");
  });
});
