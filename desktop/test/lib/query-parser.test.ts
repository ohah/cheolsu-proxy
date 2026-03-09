import { describe, test, expect } from "bun:test";

import { parseFilterQuery } from "../../src/shared/lib/query-parser";

describe("parseFilterQuery", () => {
  test("빈 쿼리 문자열은 빈 결과 반환", () => {
    const result = parseFilterQuery("");
    expect(result.methods).toEqual([]);
    expect(result.excludeMethods).toEqual([]);
    expect(result.status).toEqual([]);
    expect(result.excludeStatus).toEqual([]);
    expect(result.urls).toEqual([]);
    expect(result.excludeUrls).toEqual([]);
    expect(result.operator).toBe("and");
  });

  test("단일 method 필터 파싱", () => {
    const result = parseFilterQuery('method="GET"');
    expect(result.methods).toEqual(["GET"]);
  });

  test("복수 method 파싱 (쉼표 구분)", () => {
    const result = parseFilterQuery('method="GET,POST,PUT"');
    expect(result.methods).toEqual(["GET", "POST", "PUT"]);
  });

  test("method 대소문자 정규화", () => {
    const result = parseFilterQuery('method="get,post"');
    expect(result.methods).toEqual(["GET", "POST"]);
  });

  test("methods 키도 지원", () => {
    const result = parseFilterQuery('methods="DELETE"');
    expect(result.methods).toEqual(["DELETE"]);
  });

  test("method 제외 필터 (!= 연산자)", () => {
    const result = parseFilterQuery('method!="OPTIONS"');
    expect(result.excludeMethods).toEqual(["OPTIONS"]);
    expect(result.methods).toEqual([]);
  });

  test("status 필터 파싱", () => {
    const result = parseFilterQuery('status="2xx"');
    expect(result.status).toEqual(["2xx"]);
  });

  test("복수 status 파싱", () => {
    const result = parseFilterQuery('status="200,404,500"');
    expect(result.status).toEqual(["200", "404", "500"]);
  });

  test("status 제외 필터", () => {
    const result = parseFilterQuery('status!="5xx"');
    expect(result.excludeStatus).toEqual(["5xx"]);
  });

  test("url 포함 필터 (= 연산자)", () => {
    const result = parseFilterQuery('url="example.com"');
    expect(result.urls).toEqual(["example.com"]);
  });

  test("url 포함 필터 (|= 연산자)", () => {
    const result = parseFilterQuery('url|="payhere"');
    expect(result.urls).toEqual(["payhere"]);
  });

  test("url 제외 필터", () => {
    const result = parseFilterQuery('url!="analytics"');
    expect(result.excludeUrls).toEqual(["analytics"]);
  });

  test("복수 url 파싱 (쉼표 구분)", () => {
    const result = parseFilterQuery('url|="api,cdn"');
    expect(result.urls).toEqual(["api", "cdn"]);
  });

  test("여러 url 조건 결합", () => {
    const result = parseFilterQuery('url|="payhere" url|="hegeg"');
    expect(result.urls).toEqual(["payhere", "hegeg"]);
  });

  test("혼합 필터 조건", () => {
    const result = parseFilterQuery('method="GET,POST" status="2xx" url|="api"');
    expect(result.methods).toEqual(["GET", "POST"]);
    expect(result.status).toEqual(["2xx"]);
    expect(result.urls).toEqual(["api"]);
    expect(result.operator).toBe("and");
  });

  test("or 연산자 감지", () => {
    const result = parseFilterQuery('method="GET" or status="2xx"');
    expect(result.operator).toBe("or");
    expect(result.methods).toEqual(["GET"]);
    expect(result.status).toEqual(["2xx"]);
  });

  test("기본 연산자는 and", () => {
    const result = parseFilterQuery('method="GET" status="200"');
    expect(result.operator).toBe("and");
  });

  test("잘못된 형식은 무시", () => {
    const result = parseFilterQuery("method=GET status 200");
    expect(result.methods).toEqual([]);
    expect(result.status).toEqual([]);
  });

  test("포함과 제외 필터 혼합", () => {
    const result = parseFilterQuery('method="GET" method!="HEAD" status!="404"');
    expect(result.methods).toEqual(["GET"]);
    expect(result.excludeMethods).toEqual(["HEAD"]);
    expect(result.excludeStatus).toEqual(["404"]);
  });

  test("공백이 포함된 값 처리", () => {
    const result = parseFilterQuery('method="GET , POST"');
    expect(result.methods).toEqual(["GET", "POST"]);
  });

  // --- 연산자 공백 재현 테스트 ---

  describe("연산자와 키워드 사이 공백 처리", () => {
    test('키워드와 연산자 사이 공백: method ="GET"', () => {
      const result = parseFilterQuery('method ="GET"');
      expect(result.methods).toEqual(["GET"]);
    });

    test('연산자와 값 사이 공백: method= "GET"', () => {
      const result = parseFilterQuery('method= "GET"');
      expect(result.methods).toEqual(["GET"]);
    });

    test('양쪽 모두 공백: method = "GET"', () => {
      const result = parseFilterQuery('method = "GET"');
      expect(result.methods).toEqual(["GET"]);
    });

    test('!= 연산자 공백: method != "OPTIONS"', () => {
      const result = parseFilterQuery('method != "OPTIONS"');
      expect(result.excludeMethods).toEqual(["OPTIONS"]);
    });

    test('|= 연산자 공백: url |= "api"', () => {
      const result = parseFilterQuery('url |= "api"');
      expect(result.urls).toEqual(["api"]);
    });

    test('혼합 필터에서 공백: method = "GET" status = "2xx"', () => {
      const result = parseFilterQuery('method = "GET" status = "2xx"');
      expect(result.methods).toEqual(["GET"]);
      expect(result.status).toEqual(["2xx"]);
    });

    test('공백 있는 것과 없는 것 혼합: method="GET" status = "2xx"', () => {
      const result = parseFilterQuery('method="GET" status = "2xx"');
      expect(result.methods).toEqual(["GET"]);
      expect(result.status).toEqual(["2xx"]);
    });
  });
});
