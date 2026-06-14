import { describe, test, expect } from "bun:test";

import { parseFilterQuery } from "@/shared/lib/query-parser";

import {
  serializeBuilderState,
  parsedQueryToBuilderState,
  type BuilderState,
} from "./query-serializer";

// url|= 조건의 값들을 serialize → parse → BuilderState 재구성하여 round-trip 결과를 돌려준다.
function roundTripUrlValues(values: string[]): string[] {
  const state: BuilderState = {
    conditions: [
      {
        id: "c1",
        field: "url",
        operator: "|=",
        values,
        logicalOperator: "and",
      },
    ],
  };
  const query = serializeBuilderState(state);
  const parsed = parseFilterQuery(query);
  const rebuilt = parsedQueryToBuilderState(parsed);
  const urlCond = rebuilt.conditions.find((c) => c.field === "url" && c.operator === "|=");
  return urlCond?.values ?? [];
}

describe("query-serializer round-trip 이스케이프", () => {
  test("큰따옴표가 포함된 값이 round-trip된다(회귀)", () => {
    expect(roundTripUrlValues(['a"b'])).toEqual(['a"b']);
  });

  test("콤마가 포함된 값이 분리되지 않고 round-trip된다", () => {
    expect(roundTripUrlValues(["a,b"])).toEqual(["a,b"]);
  });

  test("정규식 백슬래시(\\d)가 보존된다", () => {
    expect(roundTripUrlValues(["\\d+"])).toEqual(["\\d+"]);
  });

  test("따옴표·콤마·정규식이 섞인 여러 값이 round-trip된다", () => {
    expect(roundTripUrlValues(['x"y', "p,q", "\\w+"])).toEqual(['x"y', "p,q", "\\w+"]);
  });
});
