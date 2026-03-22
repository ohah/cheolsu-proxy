import { describe, test, expect } from "bun:test";
import { buildGridTemplate, TABLE_COLUMNS, type ColumnKey } from "./consts";

describe("buildGridTemplate", () => {
  test("모든 컬럼이 보이면 전체 그리드 템플릿을 생성한다", () => {
    const all = new Set<ColumnKey>(TABLE_COLUMNS.map((c) => c.key));
    const result = buildGridTemplate(all);
    // checkbox(24px) + 모든 컬럼
    expect(result.split(" ")).toHaveLength(TABLE_COLUMNS.length + 1);
    expect(result).toStartWith("24px");
  });

  test("컬럼 하나만 보이면 checkbox + 해당 컬럼 크기만 반환한다", () => {
    const result = buildGridTemplate(new Set<ColumnKey>(["method"]));
    expect(result).toBe("24px 1fr");
  });

  test("빈 컬럼 셋이면 checkbox만 반환한다", () => {
    const result = buildGridTemplate(new Set<ColumnKey>());
    expect(result).toBe("24px");
  });

  test("컬럼 순서는 TABLE_COLUMNS 정의 순서를 따른다", () => {
    const cols = new Set<ColumnKey>(["status", "path"]);
    const result = buildGridTemplate(cols);
    // path(5fr)가 status(1fr)보다 먼저
    expect(result).toBe("24px 5fr 1fr");
  });
});

describe("TABLE_COLUMNS", () => {
  test("8개 컬럼이 정의되어 있다", () => {
    expect(TABLE_COLUMNS).toHaveLength(8);
  });

  test("각 컬럼에 key, label, gridSize가 있다", () => {
    for (const col of TABLE_COLUMNS) {
      expect(col.key).toBeTruthy();
      expect(col.label).toBeTruthy();
      expect(col.gridSize).toBeTruthy();
    }
  });

  test("중복된 key가 없다", () => {
    const keys = TABLE_COLUMNS.map((c) => c.key);
    expect(new Set(keys).size).toBe(keys.length);
  });
});
