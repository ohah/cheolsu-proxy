import type { ParsedQuery } from "@/shared/lib/query-parser";

export interface FilterCondition {
  id: string;
  field: "method" | "status" | "url";
  operator: "=" | "!=" | "|=";
  values: string[];
  logicalOperator: "and" | "or";
}

export interface BuilderState {
  conditions: FilterCondition[];
}

let nextId = 0;
export function createConditionId(): string {
  nextId += 1;
  return `cond_${nextId}_${Date.now()}`;
}

export function createEmptyCondition(): FilterCondition {
  return {
    id: createConditionId(),
    field: "method",
    operator: "=",
    values: [],
    logicalOperator: "and",
  };
}

/**
 * BuilderState → 쿼리 문자열
 */
export function serializeBuilderState(state: BuilderState): string {
  const activeParts: { query: string; logicalOperator: "and" | "or" }[] = [];

  for (const condition of state.conditions) {
    if (condition.values.length === 0) continue;
    const valueStr = condition.values.join(",");
    activeParts.push({
      query: `${condition.field}${condition.operator}"${valueStr}"`,
      logicalOperator: condition.logicalOperator,
    });
  }

  if (activeParts.length === 0) return "";

  let result = activeParts[0].query;
  for (let i = 1; i < activeParts.length; i++) {
    const separator = activeParts[i].logicalOperator === "or" ? " or " : " ";
    result += separator + activeParts[i].query;
  }

  return result;
}

/**
 * ParsedQuery → BuilderState
 */
export function parsedQueryToBuilderState(parsed: ParsedQuery): BuilderState {
  const conditions: FilterCondition[] = [];

  if (parsed.methods.length > 0) {
    conditions.push({
      id: createConditionId(),
      field: "method",
      operator: "=",
      values: parsed.methods,
      logicalOperator: "and",
    });
  }
  if (parsed.excludeMethods.length > 0) {
    conditions.push({
      id: createConditionId(),
      field: "method",
      operator: "!=",
      values: parsed.excludeMethods,
      logicalOperator: parsed.operator,
    });
  }
  if (parsed.status.length > 0) {
    conditions.push({
      id: createConditionId(),
      field: "status",
      operator: "=",
      values: parsed.status,
      logicalOperator: parsed.operator,
    });
  }
  if (parsed.excludeStatus.length > 0) {
    conditions.push({
      id: createConditionId(),
      field: "status",
      operator: "!=",
      values: parsed.excludeStatus,
      logicalOperator: parsed.operator,
    });
  }
  if (parsed.urls.length > 0) {
    conditions.push({
      id: createConditionId(),
      field: "url",
      operator: "|=",
      values: parsed.urls,
      logicalOperator: parsed.operator,
    });
  }
  if (parsed.excludeUrls.length > 0) {
    conditions.push({
      id: createConditionId(),
      field: "url",
      operator: "!=",
      values: parsed.excludeUrls,
      logicalOperator: parsed.operator,
    });
  }

  // 첫 번째 조건의 logicalOperator는 의미 없으므로 "and"로 설정
  if (conditions.length > 0) {
    conditions[0].logicalOperator = "and";
  }

  return { conditions };
}
