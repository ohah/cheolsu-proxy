import { describe, test, expect } from "bun:test";

import { generateCurlCommand } from "../../src/shared/lib/curl";
import { generateFetchCommand } from "../../src/shared/lib/fetch";
import { generateHttpieCommand } from "../../src/shared/lib/httpie";
import { generatePythonRequestsCommand } from "../../src/shared/lib/python-requests";
import type { HttpTransaction } from "../../src/entities/proxy/model/types";
import type { DataType } from "../../src/entities/proxy/model/data-type";

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

// ============================================================
// curl
// ============================================================
describe("generateCurlCommand", () => {
  test("request가 없으면 기본 curl 반환", () => {
    const result = generateCurlCommand(createEmptyTransaction());
    expect(result).toBe('curl -X GET "http://localhost"');
  });

  test("GET 요청 변환", () => {
    const result = generateCurlCommand(createTransaction());
    expect(result).toContain("curl -X GET");
    expect(result).toContain("https://example.com/api");
  });

  test("POST 요청 + 헤더 + JSON 바디", () => {
    const result = generateCurlCommand(
      createTransaction({
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: '{"key":"value"}',
        data_type: "Json",
      }),
    );
    expect(result).toContain("curl -X POST");
    expect(result).toContain('-H "Content-Type: application/json"');
    expect(result).toContain("-d '");
    expect(result).toContain('{"key":"value"}');
  });

  test("바디의 싱글 쿼트 이스케이프", () => {
    const result = generateCurlCommand(
      createTransaction({
        method: "POST",
        body: "it's a test",
        data_type: "Text",
      }),
    );
    expect(result).toContain("it\\'s a test");
  });
});

// ============================================================
// fetch
// ============================================================
describe("generateFetchCommand", () => {
  test("request가 없으면 기본 fetch 반환", () => {
    const result = generateFetchCommand(createEmptyTransaction());
    expect(result).toBe('fetch("http://localhost")');
  });

  test("GET 요청 - method 생략", () => {
    const result = generateFetchCommand(createTransaction());
    expect(result).not.toContain("method:");
    expect(result).toContain('fetch("https://example.com/api"');
  });

  test("POST 요청 + 헤더", () => {
    const result = generateFetchCommand(
      createTransaction({
        method: "POST",
        headers: { "Content-Type": "application/json" },
      }),
    );
    expect(result).toContain('method: "POST"');
    expect(result).toContain('"Content-Type": "application/json"');
  });

  test("POST + JSON 바디 → JSON.stringify 사용", () => {
    const result = generateFetchCommand(
      createTransaction({
        method: "POST",
        body: '{"key":"value"}',
        data_type: "Json",
      }),
    );
    expect(result).toContain("JSON.stringify(");
  });

  test("POST + 텍스트 바디", () => {
    const result = generateFetchCommand(
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
    const result = generateFetchCommand(createTransaction({ method: "PUT" }));
    expect(result).toContain('method: "PUT"');
  });

  test("URL의 특수문자 이스케이프", () => {
    const result = generateFetchCommand(
      createTransaction({ uri: 'https://example.com/api?q="test"' }),
    );
    expect(result).toContain('\\"test\\"');
  });
});

// ============================================================
// httpie
// ============================================================
describe("generateHttpieCommand", () => {
  test("request가 없으면 기본 httpie 반환", () => {
    const result = generateHttpieCommand(createEmptyTransaction());
    expect(result).toBe("http GET http://localhost");
  });

  test("GET 요청 변환", () => {
    const result = generateHttpieCommand(createTransaction());
    expect(result).toContain("http GET");
    expect(result).toContain("https://example.com/api");
  });

  test("POST 요청 + 헤더", () => {
    const result = generateHttpieCommand(
      createTransaction({
        method: "POST",
        headers: { "Content-Type": "application/json" },
      }),
    );
    expect(result).toContain("http POST");
    expect(result).toContain("Content-Type:application/json");
  });

  test("POST + JSON 바디 → key=value 형식", () => {
    const result = generateHttpieCommand(
      createTransaction({
        method: "POST",
        body: '{"name":"test","count":42}',
        data_type: "Json",
      }),
    );
    expect(result).toContain("name=test");
    expect(result).toContain("count:=42");
  });

  test("POST + 텍스트 바디 → echo 파이프", () => {
    const result = generateHttpieCommand(
      createTransaction({
        method: "POST",
        body: "plain text",
        data_type: "Text",
      }),
    );
    expect(result).toContain("echo");
    expect(result).toContain("| http POST");
  });

  test("DELETE 메서드", () => {
    const result = generateHttpieCommand(createTransaction({ method: "DELETE" }));
    expect(result).toContain("http DELETE");
  });

  test("특수문자가 포함된 URL 이스케이프", () => {
    const result = generateHttpieCommand(
      createTransaction({ uri: "https://example.com/api?q=hello world" }),
    );
    expect(result).toContain("'https://example.com/api?q=hello world'");
  });
});

// ============================================================
// python requests
// ============================================================
describe("generatePythonRequestsCommand", () => {
  test("request가 없으면 기본 코드 반환", () => {
    const result = generatePythonRequestsCommand(createEmptyTransaction());
    expect(result).toContain("import requests");
    expect(result).toContain('requests.get("http://localhost")');
  });

  test("GET 요청 변환", () => {
    const result = generatePythonRequestsCommand(createTransaction());
    expect(result).toContain("import requests");
    expect(result).toContain("requests.get(");
    expect(result).toContain("https://example.com/api");
  });

  test("POST 요청 + 헤더", () => {
    const result = generatePythonRequestsCommand(
      createTransaction({
        method: "POST",
        headers: { "Content-Type": "application/json" },
      }),
    );
    expect(result).toContain("requests.post(");
    expect(result).toContain('"Content-Type": "application/json"');
  });

  test("POST + JSON 바디 → json= 파라미터 사용", () => {
    const result = generatePythonRequestsCommand(
      createTransaction({
        method: "POST",
        body: '{"key":"value"}',
        data_type: "Json",
      }),
    );
    expect(result).toContain("json=");
    expect(result).toContain('"key": "value"');
  });

  test("POST + 텍스트 바디 → data= 파라미터 사용", () => {
    const result = generatePythonRequestsCommand(
      createTransaction({
        method: "POST",
        body: "plain text",
        data_type: "Text",
      }),
    );
    expect(result).toContain('data="plain text"');
  });

  test("PUT 메서드", () => {
    const result = generatePythonRequestsCommand(createTransaction({ method: "PUT" }));
    expect(result).toContain("requests.put(");
  });

  test("PATCH 메서드", () => {
    const result = generatePythonRequestsCommand(createTransaction({ method: "PATCH" }));
    expect(result).toContain("requests.patch(");
  });

  test("Python 문자열 이스케이프", () => {
    const result = generatePythonRequestsCommand(
      createTransaction({
        method: "POST",
        body: 'text with "quotes" and \\backslash',
        data_type: "Text",
      }),
    );
    expect(result).toContain('\\"quotes\\"');
    expect(result).toContain("\\\\backslash");
  });

  test("JSON 바디에서 boolean/null 값 Python 형식 변환", () => {
    const result = generatePythonRequestsCommand(
      createTransaction({
        method: "POST",
        body: '{"active":true,"deleted":false,"value":null}',
        data_type: "Json",
      }),
    );
    expect(result).toContain("True");
    expect(result).toContain("False");
    expect(result).toContain("None");
  });

  test("JSON 바디에서 배열 값 변환", () => {
    const result = generatePythonRequestsCommand(
      createTransaction({
        method: "POST",
        body: '{"tags":["a","b"]}',
        data_type: "Json",
      }),
    );
    expect(result).toContain('["a", "b"]');
  });
});
