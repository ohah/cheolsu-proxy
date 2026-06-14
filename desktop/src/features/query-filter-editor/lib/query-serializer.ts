import type { ParsedQuery } from "@/shared/lib/query-parser";

export interface FilterCondition {
  id: string;
  field: "method" | "status" | "url" | "client";
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
// 파서가 콤마를 구분자로 split하므로, 값 내부 콤마를 \,로 escape해야 round-trip이 깨지지 않는다.
// (백슬래시/따옴표는 기존 동작 유지를 위해 건드리지 않는다 — 정규식 패턴 \d 등 보존)
function escapeFilterValue(v: string): string {
  return v.replace(/,/g, "\\,");
}

export function serializeBuilderState(state: BuilderState): string {
  const activeParts: { query: string; logicalOperator: "and" | "or" }[] = [];

  for (const condition of state.conditions) {
    if (condition.values.length === 0) continue;
    const valueStr = condition.values.map(escapeFilterValue).join(",");
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
  if (parsed.clients.length > 0) {
    conditions.push({
      id: createConditionId(),
      field: "client",
      operator: "|=",
      values: parsed.clients,
      logicalOperator: parsed.operator,
    });
  }
  if (parsed.excludeClients.length > 0) {
    conditions.push({
      id: createConditionId(),
      field: "client",
      operator: "!=",
      values: parsed.excludeClients,
      logicalOperator: parsed.operator,
    });
  }

  // 첫 번째 조건의 logicalOperator는 의미 없으므로 "and"로 설정
  if (conditions.length > 0) {
    conditions[0].logicalOperator = "and";
  }

  return { conditions };
}
