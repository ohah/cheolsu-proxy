import { describe, test, expect } from "bun:test";

import { inferExpectSchema, inferK6Checks } from "../../src/shared/lib/schema-inference";

describe("inferExpectSchema", () => {
  test("null은 'null' 반환", () => {
    expect(inferExpectSchema(null)).toBe("null");
  });

  test("undefined은 'null' 반환", () => {
    expect(inferExpectSchema(undefined)).toBe("null");
  });

  test("숫자는 expect.any(Number)", () => {
    expect(inferExpectSchema(42)).toBe("expect.any(Number)");
  });

  test("소수점 숫자도 expect.any(Number)", () => {
    expect(inferExpectSchema(3.14)).toBe("expect.any(Number)");
  });

  test("문자열은 expect.any(String)", () => {
    expect(inferExpectSchema("hello")).toBe("expect.any(String)");
  });

  test("boolean은 expect.any(Boolean)", () => {
    expect(inferExpectSchema(true)).toBe("expect.any(Boolean)");
  });

  test("빈 배열은 expect.any(Array)", () => {
    expect(inferExpectSchema([])).toBe("expect.any(Array)");
  });

  test("빈 객체는 expect.any(Object)", () => {
    expect(inferExpectSchema({})).toBe("expect.any(Object)");
  });

  test("프리미티브 배열은 arrayContaining", () => {
    const result = inferExpectSchema([1, 2, 3]);
    expect(result).toContain("expect.arrayContaining");
    expect(result).toContain("expect.any(Number)");
  });

  test("객체 배열은 멀티라인 arrayContaining", () => {
    const result = inferExpectSchema([{ id: 1 }]);
    expect(result).toContain("expect.arrayContaining");
    expect(result).toContain("expect.objectContaining");
  });

  test("객체는 objectContaining", () => {
    const result = inferExpectSchema({ name: "test", count: 5 });
    expect(result).toContain("expect.objectContaining");
    expect(result).toContain("name: expect.any(String)");
    expect(result).toContain("count: expect.any(Number)");
  });

  test("특수 문자 키는 따옴표로 감싸기", () => {
    const result = inferExpectSchema({ "special-key": "value" });
    expect(result).toContain('"special-key"');
  });

  test("중첩 객체 처리", () => {
    const result = inferExpectSchema({ user: { name: "test" } });
    expect(result).toContain("expect.objectContaining");
    expect(result).toContain("name: expect.any(String)");
  });
});

describe("inferK6Checks", () => {
  test("null은 빈 배열 반환", () => {
    expect(inferK6Checks(null)).toEqual([]);
  });

  test("배열은 is array 체크 생성", () => {
    const result = inferK6Checks([1, 2, 3]);
    expect(result).toHaveLength(1);
    expect(result[0]).toContain("is array");
  });

  test("객체는 필드 존재 체크 생성", () => {
    const result = inferK6Checks({ id: 1, name: "test" });
    expect(result).toHaveLength(2);
    expect(result[0]).toContain(".id exists");
    expect(result[1]).toContain(".name exists");
  });

  test("8개 이상 필드는 8개까지만", () => {
    const obj: Record<string, number> = {};
    for (let i = 0; i < 12; i++) {
      obj[`field${i}`] = i;
    }
    const result = inferK6Checks(obj);
    expect(result).toHaveLength(8);
  });

  test("특수 문자 키는 bracket 표기법", () => {
    const result = inferK6Checks({ "my-key": "value" });
    expect(result[0]).toContain('["my-key"]');
  });
});
