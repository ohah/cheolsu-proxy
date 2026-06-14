import { describe, test, expect } from "bun:test";

import {
  extractBoundary,
  isMultipartFormData,
  isUrlencoded,
  parseMultipartFormData,
  parseUrlencoded,
} from "../../src/features/transaction-details/lib/form-data-parser";

// ─── extractBoundary ─────────────────────────────────────────────────

describe("extractBoundary", () => {
  test("일반적인 boundary 추출", () => {
    const ct = "multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW";
    expect(extractBoundary(ct)).toBe("----WebKitFormBoundary7MA4YWxkTrZu0gW");
  });

  test("따옴표로 감싸진 boundary 추출", () => {
    const ct = 'multipart/form-data; boundary="my-boundary-123"';
    expect(extractBoundary(ct)).toBe("my-boundary-123");
  });

  test("boundary가 없는 경우 null 반환", () => {
    expect(extractBoundary("multipart/form-data")).toBeNull();
  });

  test("빈 문자열은 null 반환", () => {
    expect(extractBoundary("")).toBeNull();
  });

  test("대소문자 구분 없이 추출", () => {
    const ct = "multipart/form-data; Boundary=abc123";
    expect(extractBoundary(ct)).toBe("abc123");
  });
});

// ─── isMultipartFormData ─────────────────────────────────────────────

describe("isMultipartFormData", () => {
  test("multipart/form-data 감지", () => {
    expect(isMultipartFormData("multipart/form-data; boundary=abc")).toBe(true);
  });

  test("대소문자 구분 없이 감지", () => {
    expect(isMultipartFormData("Multipart/Form-Data; boundary=abc")).toBe(true);
  });

  test("다른 Content-Type은 false", () => {
    expect(isMultipartFormData("application/json")).toBe(false);
  });
});

// ─── isUrlencoded ────────────────────────────────────────────────────

describe("isUrlencoded", () => {
  test("urlencoded 감지", () => {
    expect(isUrlencoded("application/x-www-form-urlencoded")).toBe(true);
  });

  test("charset 파라미터 포함 감지", () => {
    expect(isUrlencoded("application/x-www-form-urlencoded; charset=UTF-8")).toBe(true);
  });

  test("대소문자 구분 없이 감지", () => {
    expect(isUrlencoded("Application/X-WWW-Form-Urlencoded")).toBe(true);
  });

  test("다른 Content-Type은 false", () => {
    expect(isUrlencoded("application/json")).toBe(false);
  });
});

// ─── parseMultipartFormData ──────────────────────────────────────────

describe("parseMultipartFormData", () => {
  const boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";

  test("단일 텍스트 필드 파싱", () => {
    const body = [
      `------WebKitFormBoundary7MA4YWxkTrZu0gW`,
      `Content-Disposition: form-data; name="username"`,
      ``,
      `john`,
      `------WebKitFormBoundary7MA4YWxkTrZu0gW--`,
    ].join("\r\n");

    const fields = parseMultipartFormData(body, boundary);
    expect(fields).toHaveLength(1);
    expect(fields[0].name).toBe("username");
    expect(fields[0].value).toBe("john");
    expect(fields[0].isFile).toBe(false);
  });

  test("여러 텍스트 필드 파싱", () => {
    const body = [
      `------WebKitFormBoundary7MA4YWxkTrZu0gW`,
      `Content-Disposition: form-data; name="first"`,
      ``,
      `John`,
      `------WebKitFormBoundary7MA4YWxkTrZu0gW`,
      `Content-Disposition: form-data; name="last"`,
      ``,
      `Doe`,
      `------WebKitFormBoundary7MA4YWxkTrZu0gW--`,
    ].join("\r\n");

    const fields = parseMultipartFormData(body, boundary);
    expect(fields).toHaveLength(2);
    expect(fields[0].name).toBe("first");
    expect(fields[0].value).toBe("John");
    expect(fields[1].name).toBe("last");
    expect(fields[1].value).toBe("Doe");
  });

  test("파일 필드 감지", () => {
    const body = [
      `------WebKitFormBoundary7MA4YWxkTrZu0gW`,
      `Content-Disposition: form-data; name="avatar"; filename="photo.jpg"`,
      `Content-Type: image/jpeg`,
      ``,
      `<binary content>`,
      `------WebKitFormBoundary7MA4YWxkTrZu0gW--`,
    ].join("\r\n");

    const fields = parseMultipartFormData(body, boundary);
    expect(fields).toHaveLength(1);
    expect(fields[0].name).toBe("avatar");
    expect(fields[0].isFile).toBe(true);
    expect(fields[0].fileName).toBe("photo.jpg");
    expect(fields[0].contentType).toBe("image/jpeg");
    expect(fields[0].value).toBeUndefined();
    expect(fields[0].size).toBeGreaterThan(0);
  });

  test("텍스트 필드와 파일 필드 혼합", () => {
    const body = [
      `------WebKitFormBoundary7MA4YWxkTrZu0gW`,
      `Content-Disposition: form-data; name="title"`,
      ``,
      `My Document`,
      `------WebKitFormBoundary7MA4YWxkTrZu0gW`,
      `Content-Disposition: form-data; name="file"; filename="doc.pdf"`,
      `Content-Type: application/pdf`,
      ``,
      `%PDF-1.4 fake content`,
      `------WebKitFormBoundary7MA4YWxkTrZu0gW--`,
    ].join("\r\n");

    const fields = parseMultipartFormData(body, boundary);
    expect(fields).toHaveLength(2);
    expect(fields[0].isFile).toBe(false);
    expect(fields[0].name).toBe("title");
    expect(fields[1].isFile).toBe(true);
    expect(fields[1].fileName).toBe("doc.pdf");
  });

  test("빈 값을 가진 텍스트 필드", () => {
    const body = [
      `------WebKitFormBoundary7MA4YWxkTrZu0gW`,
      `Content-Disposition: form-data; name="empty_field"`,
      ``,
      ``,
      `------WebKitFormBoundary7MA4YWxkTrZu0gW--`,
    ].join("\r\n");

    const fields = parseMultipartFormData(body, boundary);
    expect(fields).toHaveLength(1);
    expect(fields[0].name).toBe("empty_field");
    expect(fields[0].value).toBe("");
    expect(fields[0].isFile).toBe(false);
  });

  test("Uint8Array 입력 처리", () => {
    const bodyStr = [
      `------WebKitFormBoundary7MA4YWxkTrZu0gW`,
      `Content-Disposition: form-data; name="key"`,
      ``,
      `value`,
      `------WebKitFormBoundary7MA4YWxkTrZu0gW--`,
    ].join("\r\n");

    const fields = parseMultipartFormData(new TextEncoder().encode(bodyStr), boundary);
    expect(fields).toHaveLength(1);
    expect(fields[0].name).toBe("key");
    expect(fields[0].value).toBe("value");
  });

  test('빈 filename은 파일로 감지 (filename="")', () => {
    const body = [
      `------WebKitFormBoundary7MA4YWxkTrZu0gW`,
      `Content-Disposition: form-data; name="file"; filename=""`,
      `Content-Type: application/octet-stream`,
      ``,
      ``,
      `------WebKitFormBoundary7MA4YWxkTrZu0gW--`,
    ].join("\r\n");

    const fields = parseMultipartFormData(body, boundary);
    expect(fields).toHaveLength(1);
    expect(fields[0].isFile).toBe(true);
    expect(fields[0].fileName).toBe("");
  });
});

// ─── parseUrlencoded ─────────────────────────────────────────────────

describe("parseUrlencoded", () => {
  test("기본 키-값 쌍 파싱", () => {
    const fields = parseUrlencoded("name=John&age=30");
    expect(fields).toHaveLength(2);
    expect(fields[0].key).toBe("name");
    expect(fields[0].value).toBe("John");
    expect(fields[1].key).toBe("age");
    expect(fields[1].value).toBe("30");
  });

  test("URL 디코딩 처리 (percent-encoding)", () => {
    const fields = parseUrlencoded("message=Hello%20World&path=%2Fhome%2Fuser");
    expect(fields).toHaveLength(2);
    expect(fields[0].key).toBe("message");
    expect(fields[0].value).toBe("Hello World");
    expect(fields[1].key).toBe("path");
    expect(fields[1].value).toBe("/home/user");
  });

  test("+ 기호를 공백으로 변환", () => {
    const fields = parseUrlencoded("query=hello+world");
    expect(fields).toHaveLength(1);
    expect(fields[0].value).toBe("hello world");
  });

  test("빈 값 처리", () => {
    const fields = parseUrlencoded("key1=&key2=value");
    expect(fields).toHaveLength(2);
    expect(fields[0].key).toBe("key1");
    expect(fields[0].value).toBe("");
    expect(fields[1].value).toBe("value");
  });

  test("값 없는 키 처리 (= 없음)", () => {
    const fields = parseUrlencoded("flag&name=test");
    expect(fields).toHaveLength(2);
    expect(fields[0].key).toBe("flag");
    expect(fields[0].value).toBe("");
    expect(fields[1].key).toBe("name");
    expect(fields[1].value).toBe("test");
  });

  test("특수문자 디코딩", () => {
    const fields = parseUrlencoded("email=user%40example.com&tag=%23trending");
    expect(fields).toHaveLength(2);
    expect(fields[0].value).toBe("user@example.com");
    expect(fields[1].value).toBe("#trending");
  });

  test("한국어 디코딩", () => {
    const fields = parseUrlencoded("name=%ED%85%8C%EC%8A%A4%ED%8A%B8");
    expect(fields).toHaveLength(1);
    expect(fields[0].value).toBe("테스트");
  });

  test("빈 문자열은 빈 배열 반환", () => {
    expect(parseUrlencoded("")).toHaveLength(0);
    expect(parseUrlencoded("   ")).toHaveLength(0);
  });

  test("Uint8Array 입력 처리", () => {
    const data = new TextEncoder().encode("key=value");
    const fields = parseUrlencoded(data);
    expect(fields).toHaveLength(1);
    expect(fields[0].key).toBe("key");
    expect(fields[0].value).toBe("value");
  });

  test("쿼리 문자열 앞의 ? 제거", () => {
    const fields = parseUrlencoded("?page=1&size=10");
    expect(fields).toHaveLength(2);
    expect(fields[0].key).toBe("page");
    expect(fields[0].value).toBe("1");
  });

  test("값에 = 기호가 포함된 경우", () => {
    const fields = parseUrlencoded("expr=1+1=2&test=ok");
    expect(fields).toHaveLength(2);
    expect(fields[0].key).toBe("expr");
    expect(fields[0].value).toBe("1 1=2");
  });

  test("잘못된 percent-encoding은 원본 반환", () => {
    const fields = parseUrlencoded("bad=%ZZ&good=ok");
    expect(fields).toHaveLength(2);
    expect(fields[0].key).toBe("bad");
    expect(fields[0].value).toBe("%ZZ");
    expect(fields[1].value).toBe("ok");
  });

  test("중복 키 허용", () => {
    const fields = parseUrlencoded("tag=a&tag=b&tag=c");
    expect(fields).toHaveLength(3);
    expect(fields.every((f) => f.key === "tag")).toBe(true);
    expect(fields.map((f) => f.value)).toEqual(["a", "b", "c"]);
  });
});

describe("parseMultipartFormData (바이너리)", () => {
  test("바이너리 파일 파트의 size가 원본 바이트 길이로 정확히 계산된다", () => {
    const boundary = "BOUNDARY";
    const enc = (s: string) => new TextEncoder().encode(s);
    // 일부 비UTF-8 바이트를 포함한 5바이트 바이너리
    const binary = new Uint8Array([0xff, 0x00, 0xfe, 0x80, 0x01]);
    const segments: Uint8Array[] = [
      enc(`--${boundary}\r\nContent-Disposition: form-data; name="field1"\r\n\r\nhello\r\n`),
      enc(
        `--${boundary}\r\nContent-Disposition: form-data; name="file1"; filename="a.bin"\r\nContent-Type: application/octet-stream\r\n\r\n`,
      ),
      binary,
      enc(`\r\n--${boundary}--\r\n`),
    ];
    const total = segments.reduce((n, s) => n + s.length, 0);
    const body = new Uint8Array(total);
    let offset = 0;
    for (const s of segments) {
      body.set(s, offset);
      offset += s.length;
    }

    const fields = parseMultipartFormData(body, boundary);
    expect(fields).toHaveLength(2);
    expect(fields[0].name).toBe("field1");
    expect(fields[0].isFile).toBe(false);
    expect(fields[0].value).toBe("hello");
    expect(fields[1].name).toBe("file1");
    expect(fields[1].isFile).toBe(true);
    expect(fields[1].fileName).toBe("a.bin");
    // UTF-8 lossy 디코딩 시 길이가 달라지지만, 바이트 단위 파서는 정확히 5를 보고한다.
    expect(fields[1].size).toBe(5);
  });
});
