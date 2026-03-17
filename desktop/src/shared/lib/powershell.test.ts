import { describe, test, expect } from "bun:test";

import { generatePowerShellCommand } from "./powershell";
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

describe("generatePowerShellCommand", () => {
  test("request가 없으면 기본 명령어 반환", () => {
    const result = generatePowerShellCommand(createEmptyTransaction());
    expect(result).toBe("Invoke-WebRequest -Uri 'http://localhost' -Method GET");
  });

  test("GET 요청 변환", () => {
    const result = generatePowerShellCommand(createTransaction());
    expect(result).toContain("Invoke-WebRequest -Uri 'https://example.com/api'");
    expect(result).toContain("-Method GET");
  });

  test("POST + JSON body → -ContentType 'application/json' -Body", () => {
    const result = generatePowerShellCommand(
      createTransaction({
        method: "POST",
        body: '{"key":"value"}',
        data_type: "Json",
      }),
    );
    expect(result).toContain("-Method POST");
    expect(result).toContain("-ContentType 'application/json'");
    expect(result).toContain("-Body '");
    expect(result).toContain('{"key":"value"}');
  });

  test("헤더가 포함된 요청 → -Headers @{ ... }", () => {
    const result = generatePowerShellCommand(
      createTransaction({
        headers: {
          "Content-Type": "application/json",
          Authorization: "Bearer token123",
        },
      }),
    );
    expect(result).toContain("-Headers @{");
    expect(result).toContain("'Content-Type' = 'application/json'");
    expect(result).toContain("'Authorization' = 'Bearer token123'");
  });

  test("헤더 값의 싱글 쿼트 이스케이프", () => {
    const result = generatePowerShellCommand(
      createTransaction({
        headers: { "X-Custom": "it's a value" },
      }),
    );
    expect(result).toContain("'it''s a value'");
  });

  test("POST + 텍스트 바디", () => {
    const result = generatePowerShellCommand(
      createTransaction({
        method: "POST",
        body: "plain text body",
        data_type: "Text",
      }),
    );
    expect(result).toContain("-Body 'plain text body'");
    expect(result).not.toContain("-ContentType");
  });

  test("PUT 메서드", () => {
    const result = generatePowerShellCommand(createTransaction({ method: "PUT" }));
    expect(result).toContain("-Method PUT");
  });

  test("DELETE 메서드", () => {
    const result = generatePowerShellCommand(createTransaction({ method: "DELETE" }));
    expect(result).toContain("-Method DELETE");
  });
});
