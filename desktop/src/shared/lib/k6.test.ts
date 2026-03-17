import { describe, test, expect } from "bun:test";

import { generateK6Code } from "./k6";
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

describe("generateK6Code", () => {
  test("request가 없으면 기본 코드 반환", () => {
    const result = generateK6Code(createEmptyTransaction());
    expect(result).toContain('import http from "k6/http"');
    expect(result).toContain("http.get");
  });

  test("GET 요청 코드 생성", () => {
    const result = generateK6Code(createTransaction());
    expect(result).toContain("http.get");
    expect(result).toContain("https://example.com/api/users");
    expect(result).toContain("status is 200");
    expect(result).toContain("sleep(1)");
  });

  test("POST + JSON body 포함", () => {
    const result = generateK6Code(
      createTransaction({
        method: "POST",
        body: '{"name":"test"}',
        data_type: "Json",
      }),
    );
    expect(result).toContain("http.post");
    expect(result).toContain("payload");
    expect(result).toContain("JSON.stringify(");
  });

  test("헤더 포함", () => {
    const result = generateK6Code(
      createTransaction({
        headers: { "Content-Type": "application/json" },
      }),
    );
    expect(result).toContain("const headers = {");
    expect(result).toContain("Content-Type");
    expect(result).toContain("{ headers }");
  });

  test("응답 JSON 필드 check 생성", () => {
    const result = generateK6Code(
      createTransaction({
        responseBodyJson: { id: 1, name: "test" },
      }),
    );
    expect(result).toContain("json()");
    expect(result).toContain("exists");
  });

  test("options 구성 포함", () => {
    const result = generateK6Code(createTransaction());
    expect(result).toContain("export const options");
    expect(result).toContain("vus: 10");
    expect(result).toContain('duration: "30s"');
  });

  test("상태 코드 반영", () => {
    const result = generateK6Code(createTransaction({ responseStatus: 500 }));
    expect(result).toContain("status is 500");
    expect(result).toContain("r.status === 500");
  });

  test("DELETE 메서드", () => {
    const result = generateK6Code(createTransaction({ method: "DELETE" }));
    expect(result).toContain("http.delete");
  });
});
