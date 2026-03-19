import { describe, test, expect } from "bun:test";
import { mock } from "bun:test";

// Tauri plugin-fs mock
mock.module("@tauri-apps/plugin-fs", () => ({
  readFile: async () => new Uint8Array(),
  BaseDirectory: { AppCache: 0 },
}));

import type { HttpTransaction } from "../../src/entities/proxy";
import { buildHarLog } from "../../src/features/har-export/lib/har-export";

function createTx(
  id: string,
  opts: {
    method?: string;
    uri?: string;
    version?: string;
    status?: number;
    reqHeaders?: Record<string, string>;
    resHeaders?: Record<string, string>;
    reqTime?: number;
    resTime?: number;
    body?: number[];
    resBody?: number[];
    reqBodySize?: number;
    resBodySize?: number;
  } = {},
): HttpTransaction {
  const reqTime = opts.reqTime ?? 1000;
  const resTime = opts.resTime ?? reqTime + 100;
  return {
    request: {
      id,
      method: opts.method ?? "GET",
      uri: opts.uri ?? `http://example.com/api/${id}`,
      version: opts.version ?? "HTTP/1.1",
      headers: opts.reqHeaders ?? {},
      body: opts.body ? new Uint8Array(opts.body) : null,
      time: reqTime,
      data_type: "Text",
      body_size: opts.reqBodySize ?? 0,
    },
    response:
      opts.status !== undefined
        ? {
            id,
            status: opts.status,
            version: opts.version ?? "HTTP/1.1",
            headers: opts.resHeaders ?? {},
            body: opts.resBody ? new Uint8Array(opts.resBody) : null,
            time: resTime,
            data_type: "Text",
            body_size: opts.resBodySize ?? 0,
          }
        : null,
  } as HttpTransaction;
}

describe("buildHarLog", () => {
  test("빈 트랜잭션 배열", async () => {
    const result = await buildHarLog([]);
    expect(result.log.version).toBe("1.2");
    expect(result.log.creator.name).toBe("Cheolsu Proxy");
    expect(result.log.entries).toHaveLength(0);
  });

  test("기본 GET 요청 변환", async () => {
    const txs = [createTx("1", { status: 200, uri: "http://example.com/api/data?key=value" })];
    const result = await buildHarLog(txs);

    expect(result.log.entries).toHaveLength(1);
    const entry = result.log.entries[0];

    expect(entry.request.method).toBe("GET");
    expect(entry.request.url).toBe("http://example.com/api/data?key=value");
    expect(entry.request.httpVersion).toBe("HTTP/1.1");
    expect(entry.response.status).toBe(200);
    expect(entry.response.statusText).toBe("OK");
  });

  test("쿼리 스트링 파싱", async () => {
    const txs = [createTx("1", { status: 200, uri: "http://example.com/api?foo=bar&baz=qux" })];
    const result = await buildHarLog(txs);
    const qs = result.log.entries[0].request.queryString;
    expect(qs).toHaveLength(2);
    expect(qs[0]).toEqual({ name: "foo", value: "bar" });
    expect(qs[1]).toEqual({ name: "baz", value: "qux" });
  });

  test("쿠키 파싱", async () => {
    const txs = [
      createTx("1", {
        status: 200,
        reqHeaders: { cookie: "session=abc123; theme=dark" },
        resHeaders: { "set-cookie": "new=value" },
      }),
    ];
    const result = await buildHarLog(txs);
    const entry = result.log.entries[0];

    expect(entry.request.cookies).toHaveLength(2);
    expect(entry.request.cookies[0].name).toBe("session");
    expect(entry.request.cookies[0].value).toBe("abc123");
    expect(entry.response.cookies).toHaveLength(1);
    expect(entry.response.cookies[0].name).toBe("new");
  });

  test("HTTP 버전 변환", async () => {
    const txs = [createTx("1", { status: 200, version: "h2" })];
    const result = await buildHarLog(txs);
    expect(result.log.entries[0].request.httpVersion).toBe("HTTP/2.0");
  });

  test("응답 없는 트랜잭션 처리", async () => {
    const txs = [createTx("1")]; // status undefined → response null
    const result = await buildHarLog(txs);
    const entry = result.log.entries[0];

    expect(entry.response.status).toBe(0);
    expect(entry.response.httpVersion).toBe("HTTP/1.1");
    expect(entry.response.content.mimeType).toBe("x-unknown");
  });

  test("요청 본문이 있는 경우 postData 포함", async () => {
    const bodyBytes = Array.from(new TextEncoder().encode('{"name":"test"}'));
    const txs = [
      createTx("1", {
        method: "POST",
        status: 201,
        body: bodyBytes,
        reqBodySize: bodyBytes.length,
        reqHeaders: { "content-type": "application/json" },
      }),
    ];
    const result = await buildHarLog(txs);
    const postData = result.log.entries[0].request.postData;

    expect(postData).toBeDefined();
    expect(postData!.mimeType).toBe("application/json");
    expect(postData!.text).toBe('{"name":"test"}');
  });

  test("경과 시간 계산", async () => {
    const txs = [createTx("1", { reqTime: 1000, resTime: 1250, status: 200 })];
    const result = await buildHarLog(txs);
    expect(result.log.entries[0].time).toBe(250);
    expect(result.log.entries[0].timings.wait).toBe(250);
  });

  test("redirect URL 설정", async () => {
    const txs = [
      createTx("1", {
        status: 302,
        resHeaders: { location: "http://example.com/new-location" },
      }),
    ];
    const result = await buildHarLog(txs);
    expect(result.log.entries[0].response.redirectURL).toBe("http://example.com/new-location");
  });

  test("다양한 상태 코드 텍스트", async () => {
    const statusMap: [number, string][] = [
      [200, "OK"],
      [201, "Created"],
      [204, "No Content"],
      [301, "Moved Permanently"],
      [400, "Bad Request"],
      [401, "Unauthorized"],
      [403, "Forbidden"],
      [404, "Not Found"],
      [500, "Internal Server Error"],
      [502, "Bad Gateway"],
      [503, "Service Unavailable"],
    ];

    for (const [status, text] of statusMap) {
      const txs = [createTx("1", { status })];
      const result = await buildHarLog(txs);
      expect(result.log.entries[0].response.statusText).toBe(text);
    }
  });

  test("request 없는 트랜잭션은 건너뜀", async () => {
    const txs = [{ request: null, response: null } as unknown as HttpTransaction];
    const result = await buildHarLog(txs);
    expect(result.log.entries).toHaveLength(0);
  });

  test("헤더 크기 계산", async () => {
    const txs = [
      createTx("1", {
        status: 200,
        reqHeaders: { "Content-Type": "application/json" },
      }),
    ];
    const result = await buildHarLog(txs);
    // "Content-Type" (12) + ": " (2) + "application/json" (16) + "\r\n" (2) + final "\r\n" (2) = 34
    expect(result.log.entries[0].request.headersSize).toBe(34);
  });
});
