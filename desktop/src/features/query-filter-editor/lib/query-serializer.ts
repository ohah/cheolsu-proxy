import type { ParsedQuery } from "@/shared/lib/query-parser";

export interface FilterCondition {
  id: string;
  field: "method" | "status" | "url";
  operator: "=" | "!=" | "|=";
  values: string[];
}

export interface BuilderState {
  conditions: FilterCondition[];
  logicalOperator: "and" | "or";
}

let nextId = 0;
export function createConditionId(): string {
  return `cond_${++nextId}_${Date.now()}`;
}

export function createEmptyCondition(): FilterCondition {
  return { id: createConditionId(), field: "method", operator: "=", values: [] };
}

/**
 * BuilderState → 쿼리 문자열
 */
export function serializeBuilderState(state: BuilderState): string {
  const parts: string[] = [];

  for (const condition of state.conditions) {
    if (condition.values.length === 0) continue;
    const valueStr = condition.values.join(",");
    parts.push(`${condition.field}${condition.operator}"${valueStr}"`);
  }

  if (parts.length === 0) return "";

  const separator = state.logicalOperator === "or" ? " or " : " ";
  return parts.join(separator);
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
    });
  }
  if (parsed.excludeMethods.length > 0) {
    conditions.push({
      id: createConditionId(),
      field: "method",
      operator: "!=",
      values: parsed.excludeMethods,
    });
  }
  if (parsed.status.length > 0) {
    conditions.push({
      id: createConditionId(),
      field: "status",
      operator: "=",
      values: parsed.status,
    });
  }
  if (parsed.excludeStatus.length > 0) {
    conditions.push({
      id: createConditionId(),
      field: "status",
      operator: "!=",
      values: parsed.excludeStatus,
    });
  }
  if (parsed.urls.length > 0) {
    conditions.push({
      id: createConditionId(),
      field: "url",
      operator: "|=",
      values: parsed.urls,
    });
  }
  if (parsed.excludeUrls.length > 0) {
    conditions.push({
      id: createConditionId(),
      field: "url",
      operator: "!=",
      values: parsed.excludeUrls,
    });
  }

  return {
    conditions,
    logicalOperator: parsed.operator,
  };
}
