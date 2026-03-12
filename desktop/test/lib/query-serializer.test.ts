import { describe, test, expect } from "bun:test";

import {
  serializeBuilderState,
  parsedQueryToBuilderState,
  createEmptyCondition,
  type BuilderState,
  type FilterCondition,
} from "../../src/features/query-filter-editor/lib/query-serializer";
import { parseFilterQuery } from "../../src/shared/lib/query-parser";

describe("serializeBuilderState", () => {
  test("빈 조건 배열은 빈 문자열 반환", () => {
    const state: BuilderState = { conditions: [], logicalOperator: "and" };
    expect(serializeBuilderState(state)).toBe("");
  });

  test("값이 없는 조건은 무시", () => {
    const state: BuilderState = {
      conditions: [{ id: "1", field: "method", operator: "=", values: [] }],
      logicalOperator: "and",
    };
    expect(serializeBuilderState(state)).toBe("");
  });

  test("단일 method 조건 직렬화", () => {
    const state: BuilderState = {
      conditions: [{ id: "1", field: "method", operator: "=", values: ["GET"] }],
      logicalOperator: "and",
    };
    expect(serializeBuilderState(state)).toBe('method="GET"');
  });

  test("복수 값이 있는 method 조건 직렬화", () => {
    const state: BuilderState = {
      conditions: [{ id: "1", field: "method", operator: "=", values: ["GET", "POST"] }],
      logicalOperator: "and",
    };
    expect(serializeBuilderState(state)).toBe('method="GET,POST"');
  });

  test("제외 연산자 직렬화", () => {
    const state: BuilderState = {
      conditions: [{ id: "1", field: "method", operator: "!=", values: ["OPTIONS"] }],
      logicalOperator: "and",
    };
    expect(serializeBuilderState(state)).toBe('method!="OPTIONS"');
  });

  test("URL contains 연산자 직렬화", () => {
    const state: BuilderState = {
      conditions: [{ id: "1", field: "url", operator: "|=", values: ["api"] }],
      logicalOperator: "and",
    };
    expect(serializeBuilderState(state)).toBe('url|="api"');
  });

  test("여러 조건 AND 직렬화", () => {
    const state: BuilderState = {
      conditions: [
        { id: "1", field: "method", operator: "=", values: ["GET"] },
        { id: "2", field: "status", operator: "=", values: ["2xx"] },
      ],
      logicalOperator: "and",
    };
    expect(serializeBuilderState(state)).toBe('method="GET" status="2xx"');
  });

  test("여러 조건 OR 직렬화", () => {
    const state: BuilderState = {
      conditions: [
        { id: "1", field: "method", operator: "=", values: ["GET"] },
        { id: "2", field: "status", operator: "=", values: ["5xx"] },
      ],
      logicalOperator: "or",
    };
    expect(serializeBuilderState(state)).toBe('method="GET" or status="5xx"');
  });
});

describe("parsedQueryToBuilderState", () => {
  test("빈 쿼리에서 빈 조건 배열 반환", () => {
    const parsed = parseFilterQuery("");
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions).toEqual([]);
    expect(state.logicalOperator).toBe("and");
  });

  test("method 포함 필터 변환", () => {
    const parsed = parseFilterQuery('method="GET,POST"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions).toHaveLength(1);
    expect(state.conditions[0].field).toBe("method");
    expect(state.conditions[0].operator).toBe("=");
    expect(state.conditions[0].values).toEqual(["GET", "POST"]);
  });

  test("method 제외 필터 변환", () => {
    const parsed = parseFilterQuery('method!="OPTIONS"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions).toHaveLength(1);
    expect(state.conditions[0].field).toBe("method");
    expect(state.conditions[0].operator).toBe("!=");
    expect(state.conditions[0].values).toEqual(["OPTIONS"]);
  });

  test("status 필터 변환", () => {
    const parsed = parseFilterQuery('status="2xx,404"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions).toHaveLength(1);
    expect(state.conditions[0].field).toBe("status");
    expect(state.conditions[0].operator).toBe("=");
    expect(state.conditions[0].values).toEqual(["2xx", "404"]);
  });

  test("url 필터 변환", () => {
    const parsed = parseFilterQuery('url|="api"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions).toHaveLength(1);
    expect(state.conditions[0].field).toBe("url");
    expect(state.conditions[0].operator).toBe("|=");
    expect(state.conditions[0].values).toEqual(["api"]);
  });

  test("혼합 필터 변환", () => {
    const parsed = parseFilterQuery('method="GET" status="2xx" url|="api"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions).toHaveLength(3);
    expect(state.conditions[0].field).toBe("method");
    expect(state.conditions[1].field).toBe("status");
    expect(state.conditions[2].field).toBe("url");
  });

  test("OR 연산자 변환", () => {
    const parsed = parseFilterQuery('method="GET" or status="5xx"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.logicalOperator).toBe("or");
  });
});

describe("라운드트립: 쿼리 → 빌더 → 쿼리", () => {
  const roundTrip = (query: string) => {
    const parsed = parseFilterQuery(query);
    const state = parsedQueryToBuilderState(parsed);
    return serializeBuilderState(state);
  };

  test('method="GET"', () => {
    expect(roundTrip('method="GET"')).toBe('method="GET"');
  });

  test('method="GET,POST" status="2xx"', () => {
    expect(roundTrip('method="GET,POST" status="2xx"')).toBe('method="GET,POST" status="2xx"');
  });

  test('method="GET" or status="5xx"', () => {
    expect(roundTrip('method="GET" or status="5xx"')).toBe('method="GET" or status="5xx"');
  });

  test('method!="OPTIONS" url|="api"', () => {
    expect(roundTrip('method!="OPTIONS" url|="api"')).toBe('method!="OPTIONS" url|="api"');
  });
});

describe("createEmptyCondition", () => {
  test("기본 빈 조건 생성", () => {
    const condition = createEmptyCondition();
    expect(condition.field).toBe("method");
    expect(condition.operator).toBe("=");
    expect(condition.values).toEqual([]);
    expect(condition.id).toBeTruthy();
  });

  test("고유 ID 생성", () => {
    const a = createEmptyCondition();
    const b = createEmptyCondition();
    expect(a.id).not.toBe(b.id);
  });
});
