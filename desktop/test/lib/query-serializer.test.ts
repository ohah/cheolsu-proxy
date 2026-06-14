import { describe, test, expect } from "bun:test";

import {
  serializeBuilderState,
  parsedQueryToBuilderState,
  createEmptyCondition,
  createConditionId,
  type BuilderState,
  type FilterCondition,
} from "../../src/features/query-filter-editor/lib/query-serializer";
import { parseFilterQuery } from "../../src/shared/lib/query-parser";

// --- 헬퍼 ---
function cond(
  overrides: Partial<FilterCondition> & Pick<FilterCondition, "field" | "values">,
): FilterCondition {
  return {
    id: overrides.id ?? createConditionId(),
    field: overrides.field,
    operator: overrides.operator ?? "=",
    values: overrides.values,
    logicalOperator: overrides.logicalOperator ?? "and",
  };
}

// ============================================================
// serializeBuilderState
// ============================================================
describe("serializeBuilderState", () => {
  // --- 기본 직렬화 ---
  test("빈 조건 배열은 빈 문자열 반환", () => {
    expect(serializeBuilderState({ conditions: [] })).toBe("");
  });

  test("값이 없는 조건은 무시", () => {
    const state: BuilderState = {
      conditions: [cond({ field: "method", values: [] })],
    };
    expect(serializeBuilderState(state)).toBe("");
  });

  test("값이 없는 조건과 있는 조건이 섞인 경우, 값이 있는 것만 직렬화", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: [] }),
        cond({ field: "status", values: ["200"] }),
        cond({ field: "url", values: [] }),
      ],
    };
    expect(serializeBuilderState(state)).toBe('status="200"');
  });

  // --- 단일 조건 ---
  test("단일 method 조건 직렬화", () => {
    const state: BuilderState = {
      conditions: [cond({ field: "method", values: ["GET"] })],
    };
    expect(serializeBuilderState(state)).toBe('method="GET"');
  });

  test("복수 값이 있는 method 조건 직렬화", () => {
    const state: BuilderState = {
      conditions: [cond({ field: "method", values: ["GET", "POST"] })],
    };
    expect(serializeBuilderState(state)).toBe('method="GET,POST"');
  });

  test("제외 연산자 직렬화", () => {
    const state: BuilderState = {
      conditions: [cond({ field: "method", operator: "!=", values: ["OPTIONS"] })],
    };
    expect(serializeBuilderState(state)).toBe('method!="OPTIONS"');
  });

  test("URL contains 연산자 직렬화", () => {
    const state: BuilderState = {
      conditions: [cond({ field: "url", operator: "|=", values: ["api"] })],
    };
    expect(serializeBuilderState(state)).toBe('url|="api"');
  });

  test("URL equals 연산자 직렬화", () => {
    const state: BuilderState = {
      conditions: [cond({ field: "url", operator: "=", values: ["example.com"] })],
    };
    expect(serializeBuilderState(state)).toBe('url="example.com"');
  });

  test("URL exclude 연산자 직렬화", () => {
    const state: BuilderState = {
      conditions: [cond({ field: "url", operator: "!=", values: ["analytics"] })],
    };
    expect(serializeBuilderState(state)).toBe('url!="analytics"');
  });

  test("status 단일 값 직렬화", () => {
    const state: BuilderState = {
      conditions: [cond({ field: "status", values: ["404"] })],
    };
    expect(serializeBuilderState(state)).toBe('status="404"');
  });

  test("status 복수 값 직렬화", () => {
    const state: BuilderState = {
      conditions: [cond({ field: "status", values: ["2xx", "3xx"] })],
    };
    expect(serializeBuilderState(state)).toBe('status="2xx,3xx"');
  });

  // --- 여러 조건 + AND/OR ---
  test("여러 조건 AND 직렬화", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: ["GET"] }),
        cond({ field: "status", values: ["2xx"], logicalOperator: "and" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe('method="GET" status="2xx"');
  });

  test("여러 조건 OR 직렬화", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: ["GET"] }),
        cond({ field: "status", values: ["5xx"], logicalOperator: "or" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe('method="GET" or status="5xx"');
  });

  test("행별 혼합 AND/OR 직렬화 (AND → OR)", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: ["GET"] }),
        cond({ field: "status", values: ["2xx"], logicalOperator: "and" }),
        cond({ field: "url", operator: "|=", values: ["api"], logicalOperator: "or" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe('method="GET" status="2xx" or url|="api"');
  });

  test("행별 혼합 AND/OR 직렬화 (OR → AND)", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: ["GET"] }),
        cond({ field: "status", values: ["5xx"], logicalOperator: "or" }),
        cond({ field: "url", operator: "|=", values: ["api"], logicalOperator: "and" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe('method="GET" or status="5xx" url|="api"');
  });

  test("3개 이상 조건 모두 OR", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: ["GET"] }),
        cond({ field: "method", values: ["POST"], logicalOperator: "or" }),
        cond({ field: "method", values: ["PUT"], logicalOperator: "or" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe('method="GET" or method="POST" or method="PUT"');
  });

  test("3개 이상 조건 모두 AND", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: ["GET"] }),
        cond({ field: "status", values: ["200"], logicalOperator: "and" }),
        cond({ field: "url", operator: "|=", values: ["api"], logicalOperator: "and" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe('method="GET" status="200" url|="api"');
  });

  // --- 빈 값 사이에 활성 조건이 있을 때 올바른 연산자 ---
  test("중간에 빈 조건이 있으면 건너뛰고 다음 조건의 연산자를 사용", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: ["GET"] }),
        cond({ field: "status", values: [], logicalOperator: "and" }),
        cond({ field: "url", operator: "|=", values: ["api"], logicalOperator: "or" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe('method="GET" or url|="api"');
  });

  // --- 첫 번째 조건의 logicalOperator는 무시 ---
  test("첫 번째 조건의 logicalOperator가 or여도 결과에 영향 없음", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: ["GET"], logicalOperator: "or" }),
        cond({ field: "status", values: ["200"], logicalOperator: "and" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe('method="GET" status="200"');
  });

  // --- 모든 필드 타입 + 연산자 조합 ---
  test("모든 필드/연산자 조합 직렬화", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", operator: "=", values: ["GET"] }),
        cond({ field: "method", operator: "!=", values: ["OPTIONS"], logicalOperator: "and" }),
        cond({ field: "status", operator: "=", values: ["2xx"], logicalOperator: "and" }),
        cond({ field: "status", operator: "!=", values: ["5xx"], logicalOperator: "and" }),
        cond({ field: "url", operator: "|=", values: ["api"], logicalOperator: "and" }),
        cond({ field: "url", operator: "!=", values: ["analytics"], logicalOperator: "and" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe(
      'method="GET" method!="OPTIONS" status="2xx" status!="5xx" url|="api" url!="analytics"',
    );
  });
});

// ============================================================
// parsedQueryToBuilderState
// ============================================================
describe("parsedQueryToBuilderState", () => {
  // --- 기본 변환 ---
  test("빈 쿼리에서 빈 조건 배열 반환", () => {
    const parsed = parseFilterQuery("");
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions).toEqual([]);
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

  test("status 제외 필터 변환", () => {
    const parsed = parseFilterQuery('status!="5xx"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions).toHaveLength(1);
    expect(state.conditions[0].field).toBe("status");
    expect(state.conditions[0].operator).toBe("!=");
    expect(state.conditions[0].values).toEqual(["5xx"]);
  });

  test("url 필터 변환", () => {
    const parsed = parseFilterQuery('url|="api"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions).toHaveLength(1);
    expect(state.conditions[0].field).toBe("url");
    expect(state.conditions[0].operator).toBe("|=");
    expect(state.conditions[0].values).toEqual(["api"]);
  });

  test("url 제외 필터 변환", () => {
    const parsed = parseFilterQuery('url!="analytics"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions).toHaveLength(1);
    expect(state.conditions[0].field).toBe("url");
    expect(state.conditions[0].operator).toBe("!=");
    expect(state.conditions[0].values).toEqual(["analytics"]);
  });

  // --- 혼합 ---
  test("혼합 필터 변환 — 필드 순서 유지", () => {
    const parsed = parseFilterQuery('method="GET" status="2xx" url|="api"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions).toHaveLength(3);
    expect(state.conditions[0].field).toBe("method");
    expect(state.conditions[1].field).toBe("status");
    expect(state.conditions[2].field).toBe("url");
  });

  test("포함과 제외가 동일 필드에 있을 때 별도 조건으로 분리", () => {
    const parsed = parseFilterQuery('method="GET" method!="HEAD"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions).toHaveLength(2);
    expect(state.conditions[0].operator).toBe("=");
    expect(state.conditions[0].values).toEqual(["GET"]);
    expect(state.conditions[1].operator).toBe("!=");
    expect(state.conditions[1].values).toEqual(["HEAD"]);
  });

  // --- logicalOperator 설정 ---
  test("AND 쿼리 — 모든 조건의 logicalOperator가 and", () => {
    const parsed = parseFilterQuery('method="GET" status="2xx" url|="api"');
    const state = parsedQueryToBuilderState(parsed);
    for (const c of state.conditions) {
      expect(c.logicalOperator).toBe("and");
    }
  });

  test("OR 쿼리 — 첫 번째는 and, 이후는 or", () => {
    const parsed = parseFilterQuery('method="GET" or status="5xx"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions[0].logicalOperator).toBe("and");
    expect(state.conditions[1].logicalOperator).toBe("or");
  });

  test("OR 쿼리 3개 조건 — 첫 번째만 and, 나머지 or", () => {
    const parsed = parseFilterQuery('method="GET" or status="5xx" or url|="api"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions[0].logicalOperator).toBe("and");
    expect(state.conditions[1].logicalOperator).toBe("or");
    expect(state.conditions[2].logicalOperator).toBe("or");
  });

  test("첫 번째 조건의 logicalOperator는 항상 and", () => {
    const parsed = parseFilterQuery('method="GET"');
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions[0].logicalOperator).toBe("and");
  });

  // --- 각 조건에 고유 ID ---
  test("각 조건에 고유 ID가 부여됨", () => {
    const parsed = parseFilterQuery('method="GET" status="200" url|="api"');
    const state = parsedQueryToBuilderState(parsed);
    const ids = state.conditions.map((c) => c.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  // --- 복잡한 혼합 쿼리 ---
  test("모든 필드 타입 포함/제외 혼합 변환", () => {
    const parsed = parseFilterQuery(
      'method="GET" method!="HEAD" status="2xx" status!="500" url|="api" url!="analytics"',
    );
    const state = parsedQueryToBuilderState(parsed);
    expect(state.conditions).toHaveLength(6);
    expect(state.conditions.map((c) => `${c.field}${c.operator}`)).toEqual([
      "method=",
      "method!=",
      "status=",
      "status!=",
      "url|=",
      "url!=",
    ]);
  });
});

// ============================================================
// 라운드트립: 쿼리 → 빌더 → 쿼리
// ============================================================
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

  test('status="2xx,3xx"', () => {
    expect(roundTrip('status="2xx,3xx"')).toBe('status="2xx,3xx"');
  });

  test('url!="analytics"', () => {
    expect(roundTrip('url!="analytics"')).toBe('url!="analytics"');
  });

  test("빈 쿼리 라운드트립", () => {
    expect(roundTrip("")).toBe("");
  });

  test("모든 필드 포함 복합 쿼리", () => {
    const q = 'method="GET" method!="HEAD" status="2xx" status!="500" url|="api" url!="analytics"';
    expect(roundTrip(q)).toBe(q);
  });

  test("OR 연산자 포함 복합 쿼리", () => {
    expect(roundTrip('method="GET" or status="5xx" or url|="error"')).toBe(
      'method="GET" or status="5xx" or url|="error"',
    );
  });
});

// ============================================================
// createEmptyCondition / createConditionId
// ============================================================
describe("createEmptyCondition", () => {
  test("기본 빈 조건 생성", () => {
    const condition = createEmptyCondition();
    expect(condition.field).toBe("method");
    expect(condition.operator).toBe("=");
    expect(condition.values).toEqual([]);
    expect(condition.logicalOperator).toBe("and");
    expect(condition.id).toBeTruthy();
  });

  test("고유 ID 생성", () => {
    const a = createEmptyCondition();
    const b = createEmptyCondition();
    expect(a.id).not.toBe(b.id);
  });

  test("여러 개 생성 시 모두 독립적 참조", () => {
    const a = createEmptyCondition();
    const b = createEmptyCondition();
    a.values.push("GET");
    expect(b.values).toEqual([]);
  });
});

describe("createConditionId", () => {
  test("문자열 반환", () => {
    expect(typeof createConditionId()).toBe("string");
  });

  test("연속 호출 시 고유 ID", () => {
    const ids = new Set(Array.from({ length: 100 }, () => createConditionId()));
    expect(ids.size).toBe(100);
  });

  test("cond_ 접두사 포함", () => {
    expect(createConditionId()).toMatch(/^cond_/);
  });
});

// ============================================================
// 행별 AND/OR 혼합 시나리오 (per-row logicalOperator)
// ============================================================
describe("행별 AND/OR 혼합 시나리오", () => {
  test("AND-AND-AND 패턴", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: ["GET"] }),
        cond({ field: "status", values: ["200"], logicalOperator: "and" }),
        cond({ field: "url", operator: "|=", values: ["api"], logicalOperator: "and" }),
        cond({ field: "url", operator: "!=", values: ["test"], logicalOperator: "and" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe('method="GET" status="200" url|="api" url!="test"');
  });

  test("OR-OR-OR 패턴", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: ["GET"] }),
        cond({ field: "method", values: ["POST"], logicalOperator: "or" }),
        cond({ field: "method", values: ["PUT"], logicalOperator: "or" }),
        cond({ field: "method", values: ["DELETE"], logicalOperator: "or" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe(
      'method="GET" or method="POST" or method="PUT" or method="DELETE"',
    );
  });

  test("AND-OR-AND 패턴", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: ["GET"] }),
        cond({ field: "status", values: ["200"], logicalOperator: "and" }),
        cond({ field: "url", operator: "|=", values: ["cdn"], logicalOperator: "or" }),
        cond({ field: "url", operator: "!=", values: ["test"], logicalOperator: "and" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe(
      'method="GET" status="200" or url|="cdn" url!="test"',
    );
  });

  test("OR-AND-OR 패턴", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: ["GET"] }),
        cond({ field: "status", values: ["5xx"], logicalOperator: "or" }),
        cond({ field: "url", operator: "|=", values: ["error"], logicalOperator: "and" }),
        cond({ field: "method", values: ["POST"], logicalOperator: "or" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe(
      'method="GET" or status="5xx" url|="error" or method="POST"',
    );
  });

  test("단일 조건일 때 logicalOperator 무관", () => {
    const stateAnd: BuilderState = {
      conditions: [cond({ field: "method", values: ["GET"], logicalOperator: "and" })],
    };
    const stateOr: BuilderState = {
      conditions: [cond({ field: "method", values: ["GET"], logicalOperator: "or" })],
    };
    expect(serializeBuilderState(stateAnd)).toBe(serializeBuilderState(stateOr));
  });
});

// ============================================================
// 엣지 케이스
// ============================================================
describe("엣지 케이스", () => {
  test("모든 조건의 값이 비어있으면 빈 문자열", () => {
    const state: BuilderState = {
      conditions: [
        cond({ field: "method", values: [] }),
        cond({ field: "status", values: [], logicalOperator: "or" }),
        cond({ field: "url", operator: "|=", values: [], logicalOperator: "and" }),
      ],
    };
    expect(serializeBuilderState(state)).toBe("");
  });

  test("URL 값에 특수문자 포함", () => {
    const state: BuilderState = {
      conditions: [cond({ field: "url", operator: "|=", values: ["/users/\\d+"] })],
    };
    expect(serializeBuilderState(state)).toBe('url|="/users/\\d+"');
  });

  test("URL 단일 값에 콤마가 포함되면 \\,로 이스케이프된다", () => {
    const state: BuilderState = {
      conditions: [cond({ field: "url", operator: "|=", values: ["api,cdn"] })],
    };
    // 콤마가 포함된 "단일 값"이므로 구분자와 구별되도록 \,로 이스케이프되어야 한다
    expect(serializeBuilderState(state)).toBe('url|="api\\,cdn"');
  });

  test("status 값이 와일드카드 패턴 (2xx, 3xx 등)", () => {
    const state: BuilderState = {
      conditions: [cond({ field: "status", values: ["2xx", "3xx", "4xx", "5xx"] })],
    };
    expect(serializeBuilderState(state)).toBe('status="2xx,3xx,4xx,5xx"');
  });

  test("많은 조건 (10개) 직렬화", () => {
    const conditions: FilterCondition[] = Array.from({ length: 10 }, (_, i) =>
      cond({
        field: "method",
        values: ["GET"],
        logicalOperator: i === 0 ? "and" : i % 2 === 0 ? "and" : "or",
      }),
    );
    const state: BuilderState = { conditions };
    const result = serializeBuilderState(state);
    // 10개 조건이 모두 직렬화되었는지 확인
    const methodCount = (result.match(/method=/g) ?? []).length;
    expect(methodCount).toBe(10);
  });

  test("parsedQueryToBuilderState 후 BuilderState에 logicalOperator 필드 없음", () => {
    const parsed = parseFilterQuery('method="GET"');
    const state = parsedQueryToBuilderState(parsed);
    // BuilderState에는 conditions만 있어야 함
    expect(Object.keys(state)).toEqual(["conditions"]);
  });

  test("콤마가 포함된 단일 값이 round-trip에서 분해되지 않음", () => {
    const state: BuilderState = {
      conditions: [cond({ field: "url", operator: "|=", values: ["https://a.com/?x=1,2"] })],
    };
    const query = serializeBuilderState(state);
    const parsed = parseFilterQuery(query);
    // 콤마가 있어도 단일 URL 값으로 복원되어야 함
    expect(parsed.urls).toEqual(["https://a.com/?x=1,2"]);
  });
});
