import { describe, test, expect } from "bun:test";
import { getAuthority } from "./utils";

describe("getAuthority", () => {
  test("HTTP URL에서 호스트명을 추출한다", () => {
    expect(getAuthority("http://example.com/path")).toBe("example.com");
  });

  test("HTTPS URL에서 호스트명을 추출한다", () => {
    expect(getAuthority("https://api.example.com/v1/users")).toBe("api.example.com");
  });

  test("포트가 포함된 URL에서 호스트:포트를 추출한다", () => {
    expect(getAuthority("http://localhost:8080/api")).toBe("localhost:8080");
  });

  test("CONNECT 요청의 host:port 형식을 그대로 반환한다", () => {
    expect(getAuthority("example.com:443")).toBe("example.com:443");
  });

  test("기본 포트(80, 443)는 포트 없이 호스트명만 반환한다", () => {
    expect(getAuthority("https://example.com:443/path")).toBe("example.com");
    expect(getAuthority("http://example.com:80/path")).toBe("example.com");
  });

  test("비표준 포트는 포트를 포함해서 반환한다", () => {
    expect(getAuthority("https://example.com:8443/path")).toBe("example.com:8443");
  });

  test("파싱 불가능한 URI는 슬래시 앞부분을 반환한다", () => {
    expect(getAuthority("invalid-uri/some/path")).toBe("invalid-uri");
  });
});
