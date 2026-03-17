import { create } from "zustand";
import type { GraphqlOperation, GraphqlOperationType, GraphqlError } from "@/entities/graphql";
import type { HttpTransaction } from "@/entities/proxy";

interface GraphqlStoreState {
  operations: GraphqlOperation[];
  selectedOperation: GraphqlOperation | null;

  setSelectedOperation: (op: GraphqlOperation | null) => void;
  clearAll: () => void;
}

export const useGraphqlStore = create<GraphqlStoreState>()((set) => ({
  operations: [],
  selectedOperation: null,

  setSelectedOperation: (op) => set({ selectedOperation: op }),

  clearAll: () =>
    set({
      operations: [],
      selectedOperation: null,
    }),
}));

/** query 문자열에서 오퍼레이션 타입을 파싱 */
function parseOperationType(query: string): GraphqlOperationType {
  const trimmed = query.trim();
  if (trimmed.startsWith("mutation")) return "mutation";
  if (trimmed.startsWith("subscription")) return "subscription";
  return "query";
}

/** query 문자열에서 오퍼레이션 이름을 파싱 */
function parseOperationName(query: string): string | null {
  const trimmed = query.trim();
  let rest: string | null = null;

  if (trimmed.startsWith("query")) rest = trimmed.slice(5);
  else if (trimmed.startsWith("mutation")) rest = trimmed.slice(8);
  else if (trimmed.startsWith("subscription")) rest = trimmed.slice(12);

  if (rest === null) return null;

  rest = rest.trimStart();
  if (rest.startsWith("{") || rest.startsWith("(") || rest.length === 0) return null;

  const match = rest.match(/^([a-zA-Z_]\w*)/);
  return match ? match[1] : null;
}

/** HttpTransaction에서 GraphQL 오퍼레이션을 추출 */
export function extractGraphqlOperations(tx: HttpTransaction): GraphqlOperation[] {
  const req = tx.request;
  if (!req || req.data_type !== "GraphQL") return [];

  const bodyJson = req.body_json as
    | Record<string, unknown>
    | Record<string, unknown>[]
    | null
    | undefined;
  if (!bodyJson) return [];

  const operations: Record<string, unknown>[] = Array.isArray(bodyJson) ? bodyJson : [bodyJson];
  const isBatch = Array.isArray(bodyJson) && bodyJson.length > 1;

  // 응답 파싱
  const resJson = tx.response?.body_json as
    | Record<string, unknown>
    | Record<string, unknown>[]
    | null
    | undefined;
  const responses: Record<string, unknown>[] = resJson
    ? Array.isArray(resJson)
      ? resJson
      : [resJson]
    : [];

  return operations
    .map((op, index) => {
      const query = (op.query as string) || "";
      const operationName = (op.operationName as string) || parseOperationName(query);
      const operationType = parseOperationType(query);
      const variables = op.variables ?? null;

      const resp = responses[index];
      const responseData = resp?.data ?? null;
      const responseErrors = (resp?.errors as GraphqlError[] | undefined) ?? null;
      const responseExtensions = resp?.extensions ?? null;

      return {
        id: isBatch ? `${req.id}-${index}` : req.id,
        operationType,
        operationName: operationName || null,
        query,
        variables,
        endpoint: req.uri,
        requestTime: req.time,
        responseTime: tx.response?.time ?? null,
        responseData,
        responseErrors,
        responseExtensions,
        statusCode: tx.response?.status ?? null,
        batchIndex: isBatch ? index : null,
        batchSize: isBatch ? operations.length : null,
      };
    })
    .filter((op) => op.query.length > 0);
}
