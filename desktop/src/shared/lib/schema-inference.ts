/**
 * JSON 값으로부터 테스트 assertion용 타입 스키마를 추론
 */
export const inferExpectSchema = (value: unknown, indent = 4): string => {
  const pad = " ".repeat(indent);
  const innerPad = " ".repeat(indent + 2);

  if (value === null || value === undefined) {
    return "null";
  }

  if (typeof value === "number") {
    return Number.isInteger(value) ? "expect.any(Number)" : "expect.any(Number)";
  }

  if (typeof value === "string") {
    return "expect.any(String)";
  }

  if (typeof value === "boolean") {
    return "expect.any(Boolean)";
  }

  if (Array.isArray(value)) {
    if (value.length === 0) {
      return "expect.any(Array)";
    }
    const itemSchema = inferExpectSchema(value[0], indent + 2);
    if (typeof value[0] === "object" && value[0] !== null && !Array.isArray(value[0])) {
      return `expect.arrayContaining([\n${innerPad}${itemSchema}\n${pad}])`;
    }
    return `expect.arrayContaining([${itemSchema}])`;
  }

  if (typeof value === "object") {
    const entries = Object.entries(value);
    if (entries.length === 0) {
      return "expect.any(Object)";
    }
    const fields = entries
      .map(([key, val]) => {
        const safeKey = /^[a-zA-Z_$][a-zA-Z0-9_$]*$/.test(key) ? key : `"${key}"`;
        return `${innerPad}${safeKey}: ${inferExpectSchema(val, indent + 2)}`;
      })
      .join(",\n");
    return `expect.objectContaining({\n${fields}\n${pad}})`;
  }

  return "expect.anything()";
};

/**
 * JSON 값으로부터 k6 check용 타입 체크 코드를 생성
 */
export const inferK6Checks = (value: unknown, path = "json"): string[] => {
  const checks: string[] = [];

  if (value === null || value === undefined) {
    return checks;
  }

  if (Array.isArray(value)) {
    checks.push(`  "${path} is array": (r) => Array.isArray(r.${path}())`);
    return checks;
  }

  if (typeof value === "object") {
    const entries = Object.entries(value);
    for (const [key] of entries.slice(0, 8)) {
      const accessor = /^[a-zA-Z_$][a-zA-Z0-9_$]*$/.test(key) ? `.${key}` : `["${key}"]`;
      checks.push(`  "${path}()${accessor} exists": (r) => r.${path}()${accessor} !== undefined`);
    }
    return checks;
  }

  return checks;
};
