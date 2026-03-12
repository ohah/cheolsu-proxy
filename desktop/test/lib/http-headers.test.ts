import { describe, it, expect } from "vitest";
import { sanitizeHopByHopHeaders } from "../../src/shared/lib/http-headers";

describe("sanitizeHopByHopHeaders", () => {
  it("hop-by-hop 헤더를 제거한다", () => {
    const headers = {
      "Content-Type": "application/json",
      Host: "example.com",
      Connection: "keep-alive",
      "Keep-Alive": "timeout=5",
      "X-Custom": "value",
    };
    const result = sanitizeHopByHopHeaders(headers);
    expect(result).toEqual({
      "Content-Type": "application/json",
      "X-Custom": "value",
    });
  });

  it("대소문자를 구분하지 않고 제거한다", () => {
    const headers = {
      HOST: "example.com",
      connection: "close",
      "PROXY-AUTHORIZATION": "Basic abc",
      "Transfer-Encoding": "chunked",
      Accept: "text/html",
    };
    const result = sanitizeHopByHopHeaders(headers);
    expect(result).toEqual({
      Accept: "text/html",
    });
  });

  it("hop-by-hop 헤더가 없으면 모든 헤더를 유지한다", () => {
    const headers = {
      "Content-Type": "text/plain",
      Accept: "*/*",
      Authorization: "Bearer token",
    };
    const result = sanitizeHopByHopHeaders(headers);
    expect(result).toEqual(headers);
  });

  it("빈 객체를 처리한다", () => {
    const result = sanitizeHopByHopHeaders({});
    expect(result).toEqual({});
  });

  it("모든 hop-by-hop 헤더를 제거한다", () => {
    const headers = {
      host: "a",
      connection: "b",
      "keep-alive": "c",
      "proxy-authenticate": "d",
      "proxy-authorization": "e",
      te: "f",
      trailers: "g",
      "transfer-encoding": "h",
      upgrade: "i",
    };
    const result = sanitizeHopByHopHeaders(headers);
    expect(result).toEqual({});
  });

  it("원본 객체를 변경하지 않는다", () => {
    const headers = { Host: "example.com", Accept: "text/html" };
    const original = { ...headers };
    sanitizeHopByHopHeaders(headers);
    expect(headers).toEqual(original);
  });
});
