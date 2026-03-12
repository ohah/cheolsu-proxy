import { describe, test, expect } from "bun:test";

import { parseFilterQuery } from "../../src/shared/lib/query-parser";

describe("parseFilterQuery", () => {
  // --- 기본 ---
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

  test("공백만 있는 쿼리는 빈 결과 반환", () => {
    const result = parseFilterQuery("   ");
    expect(result.methods).toEqual([]);
    expect(result.operator).toBe("and");
  });

  // --- method ---
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

  test("method 포함과 제외 동시 사용", () => {
    const result = parseFilterQuery('method="GET,POST" method!="HEAD"');
    expect(result.methods).toEqual(["GET", "POST"]);
    expect(result.excludeMethods).toEqual(["HEAD"]);
  });

  test("여러 개의 method 포함 필터 병합", () => {
    const result = parseFilterQuery('method="GET" method="POST"');
    expect(result.methods).toEqual(["GET", "POST"]);
  });

  test("모든 HTTP 메서드 파싱", () => {
    const result = parseFilterQuery('method="GET,POST,PUT,DELETE,PATCH,HEAD,OPTIONS"');
    expect(result.methods).toEqual(["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]);
  });

  // --- status ---
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

  test("status 포함과 제외 동시 사용", () => {
    const result = parseFilterQuery('status="2xx" status!="5xx"');
    expect(result.status).toEqual(["2xx"]);
    expect(result.excludeStatus).toEqual(["5xx"]);
  });

  test("status 와일드카드 패턴 모두 지원", () => {
    const result = parseFilterQuery('status="1xx,2xx,3xx,4xx,5xx"');
    expect(result.status).toEqual(["1xx", "2xx", "3xx", "4xx", "5xx"]);
  });

  // --- url ---
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

  test("url 포함과 제외 동시 사용", () => {
    const result = parseFilterQuery('url|="api" url!="analytics"');
    expect(result.urls).toEqual(["api"]);
    expect(result.excludeUrls).toEqual(["analytics"]);
  });

  test("url 값에 경로 포함", () => {
    const result = parseFilterQuery('url|="/api/v1/users"');
    expect(result.urls).toEqual(["/api/v1/users"]);
  });

  test("url 값에 정규식 패턴 포함", () => {
    const result = parseFilterQuery('url|="/users/\\d+"');
    expect(result.urls).toEqual(["/users/\\d+"]);
  });

  // --- 혼합 필터 ---
  test("혼합 필터 조건", () => {
    const result = parseFilterQuery('method="GET,POST" status="2xx" url|="api"');
    expect(result.methods).toEqual(["GET", "POST"]);
    expect(result.status).toEqual(["2xx"]);
    expect(result.urls).toEqual(["api"]);
    expect(result.operator).toBe("and");
  });

  test("포함과 제외 필터 혼합", () => {
    const result = parseFilterQuery('method="GET" method!="HEAD" status!="404"');
    expect(result.methods).toEqual(["GET"]);
    expect(result.excludeMethods).toEqual(["HEAD"]);
    expect(result.excludeStatus).toEqual(["404"]);
  });

  test("모든 필드 포함/제외 혼합", () => {
    const result = parseFilterQuery(
      'method="GET" method!="HEAD" status="2xx" status!="500" url|="api" url!="analytics"',
    );
    expect(result.methods).toEqual(["GET"]);
    expect(result.excludeMethods).toEqual(["HEAD"]);
    expect(result.status).toEqual(["2xx"]);
    expect(result.excludeStatus).toEqual(["500"]);
    expect(result.urls).toEqual(["api"]);
    expect(result.excludeUrls).toEqual(["analytics"]);
  });

  // --- 연산자 감지 ---
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

  test("OR 대소문자 무관 감지", () => {
    const result = parseFilterQuery('method="GET" OR status="2xx"');
    expect(result.operator).toBe("or");
  });

  test("or이 여러 번 나와도 정상 파싱", () => {
    const result = parseFilterQuery('method="GET" or status="200" or url|="api"');
    expect(result.operator).toBe("or");
    expect(result.methods).toEqual(["GET"]);
    expect(result.status).toEqual(["200"]);
    expect(result.urls).toEqual(["api"]);
  });

  // --- 잘못된 형식 ---
  test("잘못된 형식은 무시", () => {
    const result = parseFilterQuery("method=GET status 200");
    expect(result.methods).toEqual([]);
    expect(result.status).toEqual([]);
  });

  test("따옴표 없는 값은 무시", () => {
    const result = parseFilterQuery("method=GET");
    expect(result.methods).toEqual([]);
  });

  test("알 수 없는 키는 무시", () => {
    const result = parseFilterQuery('host="example.com"');
    expect(result.methods).toEqual([]);
    expect(result.status).toEqual([]);
    expect(result.urls).toEqual([]);
  });

  test("부분적으로 유효한 쿼리는 유효한 부분만 파싱", () => {
    const result = parseFilterQuery('method="GET" invalid status="200"');
    expect(result.methods).toEqual(["GET"]);
    expect(result.status).toEqual(["200"]);
  });

  // --- 공백 처리 ---
  test("공백이 포함된 값 처리", () => {
    const result = parseFilterQuery('method="GET , POST"');
    expect(result.methods).toEqual(["GET", "POST"]);
  });

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

  // --- 빈 값 ---
  describe("빈 값 처리", () => {
    test('빈 따옴표: method=""', () => {
      const result = parseFilterQuery('method=""');
      expect(result.methods).toEqual([]);
    });

    test('빈 따옴표와 유효한 조건 혼합: method="" status="200"', () => {
      const result = parseFilterQuery('method="" status="200"');
      expect(result.methods).toEqual([]);
      expect(result.status).toEqual(["200"]);
    });
  });

  // --- 따옴표 변형 ---
  describe("다양한 따옴표 지원", () => {
    test("작은따옴표: method='GET'", () => {
      const result = parseFilterQuery("method='GET'");
      expect(result.methods).toEqual(["GET"]);
    });

    test("백틱: method=`GET`", () => {
      const result = parseFilterQuery("method=`GET`");
      expect(result.methods).toEqual(["GET"]);
    });

    test("작은따옴표로 복수 값: method='GET,POST'", () => {
      const result = parseFilterQuery("method='GET,POST'");
      expect(result.methods).toEqual(["GET", "POST"]);
    });

    test("백틱으로 복수 값: status=`200,404`", () => {
      const result = parseFilterQuery("status=`200,404`");
      expect(result.status).toEqual(["200", "404"]);
    });

    test("작은따옴표로 URL: url|='api'", () => {
      const result = parseFilterQuery("url|='api'");
      expect(result.urls).toEqual(["api"]);
    });

    test("백틱으로 URL: url|=`api`", () => {
      const result = parseFilterQuery("url|=`api`");
      expect(result.urls).toEqual(["api"]);
    });

    test("작은따옴표 제외 연산자: method!='OPTIONS'", () => {
      const result = parseFilterQuery("method!='OPTIONS'");
      expect(result.excludeMethods).toEqual(["OPTIONS"]);
    });

    test("백틱 제외 연산자: status!=`5xx`", () => {
      const result = parseFilterQuery("status!=`5xx`");
      expect(result.excludeStatus).toEqual(["5xx"]);
    });

    test("서로 다른 따옴표 혼합 사용", () => {
      const result = parseFilterQuery("method=\"GET\" status='2xx' url|=`api`");
      expect(result.methods).toEqual(["GET"]);
      expect(result.status).toEqual(["2xx"]);
      expect(result.urls).toEqual(["api"]);
    });

    test("빈 작은따옴표: method=''", () => {
      const result = parseFilterQuery("method=''");
      expect(result.methods).toEqual([]);
    });

    test("빈 백틱: method=``", () => {
      const result = parseFilterQuery("method=``");
      expect(result.methods).toEqual([]);
    });

    test("따옴표 혼합 + OR 연산자", () => {
      const result = parseFilterQuery("method='GET' or status=\"5xx\"");
      expect(result.operator).toBe("or");
      expect(result.methods).toEqual(["GET"]);
      expect(result.status).toEqual(["5xx"]);
    });

    test("따옴표 혼합 + 공백 있는 연산자", () => {
      const result = parseFilterQuery("method = 'GET' status = `2xx`");
      expect(result.methods).toEqual(["GET"]);
      expect(result.status).toEqual(["2xx"]);
    });
  });

  // --- 이스케이프 따옴표 ---
  describe("이스케이프 따옴표 처리", () => {
    test('큰따옴표 이스케이프: url|="path/with\\"quote"', () => {
      const result = parseFilterQuery('url|="path/with\\"quote"');
      expect(result.urls).toEqual(['path/with"quote']);
    });

    test("작은따옴표 이스케이프: url|='it\\'s'", () => {
      const result = parseFilterQuery("url|='it\\'s'");
      expect(result.urls).toEqual(["it's"]);
    });

    test("백틱 이스케이프: url|=`path/with\\`tick`", () => {
      const result = parseFilterQuery("url|=`path/with\\`tick`");
      expect(result.urls).toEqual(["path/with`tick"]);
    });

    test('백슬래시 이스케이프: url|="path\\\\dir"', () => {
      const result = parseFilterQuery('url|="path\\\\dir"');
      expect(result.urls).toEqual(["path\\dir"]);
    });

    test("이스케이프된 값과 일반 값 혼합", () => {
      const result = parseFilterQuery('method="GET" url|="path/with\\"quote"');
      expect(result.methods).toEqual(["GET"]);
      expect(result.urls).toEqual(['path/with"quote']);
    });
  });
});
