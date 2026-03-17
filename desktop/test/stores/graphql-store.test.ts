import { describe, test, expect, beforeEach } from "bun:test";
import "../../test/mocks/tauri";

import type { HttpTransaction } from "../../src/entities/proxy";
import type { GraphqlOperation } from "../../src/entities/graphql";

async function getStore() {
  const { useGraphqlStore } = await import("../../src/shared/stores/graphql-store");
  return useGraphqlStore;
}

async function getExtractor() {
  const { extractGraphqlOperations } = await import("../../src/shared/stores/graphql-store");
  return extractGraphqlOperations;
}

function createTransaction(overrides: Partial<HttpTransaction> = {}): HttpTransaction {
  return {
    request: {
      method: "POST",
      uri: "https://api.example.com/graphql",
      version: "HTTP/1.1",
      headers: { "content-type": "application/json" },
      body: null,
      time: 1700000000000000000,
      id: "tx-1",
      data_type: "GraphQL",
      body_json: { query: "query GetUser { user { name } }", operationName: "GetUser", variables: { id: "1" } },
      body_size: 100,
    },
    response: {
      status: 200,
      version: "HTTP/1.1",
      headers: { "content-type": "application/json" },
      body: null,
      time: 1700000000100000000,
      id: "tx-1",
      data_type: "JSON",
      body_json: { data: { user: { name: "Alice" } } },
      body_size: 50,
    },
    ...overrides,
  };
}

describe("graphql-store", () => {
  let store: Awaited<ReturnType<typeof getStore>>;

  beforeEach(async () => {
    store = await getStore();
    store.setState({
      operations: [],
      selectedOperation: null,
    });
  });

  test("초기 상태", () => {
    const state = store.getState();
    expect(state.operations).toEqual([]);
    expect(state.selectedOperation).toBeNull();
  });

  test("setSelectedOperation: 오퍼레이션 선택", () => {
    const op: GraphqlOperation = {
      id: "op-1",
      operationType: "query",
      operationName: "GetUser",
      query: "query GetUser { user { name } }",
      variables: null,
      endpoint: "https://api.example.com/graphql",
      requestTime: 1700000000000000000,
      responseTime: null,
      responseData: null,
      responseErrors: null,
      responseExtensions: null,
      statusCode: null,
      batchIndex: null,
      batchSize: null,
    };
    store.getState().setSelectedOperation(op);
    expect(store.getState().selectedOperation).toEqual(op);
  });

  test("setSelectedOperation: null로 선택 해제", () => {
    const op: GraphqlOperation = {
      id: "op-1",
      operationType: "query",
      operationName: "GetUser",
      query: "query GetUser { user { name } }",
      variables: null,
      endpoint: "https://api.example.com/graphql",
      requestTime: 1700000000000000000,
      responseTime: null,
      responseData: null,
      responseErrors: null,
      responseExtensions: null,
      statusCode: null,
      batchIndex: null,
      batchSize: null,
    };
    store.getState().setSelectedOperation(op);
    store.getState().setSelectedOperation(null);
    expect(store.getState().selectedOperation).toBeNull();
  });

  test("clearAll: 전체 초기화", () => {
    store.getState().setSelectedOperation({
      id: "op-1",
      operationType: "query",
      operationName: null,
      query: "{ user }",
      variables: null,
      endpoint: "",
      requestTime: 0,
      responseTime: null,
      responseData: null,
      responseErrors: null,
      responseExtensions: null,
      statusCode: null,
      batchIndex: null,
      batchSize: null,
    });
    store.getState().clearAll();
    const state = store.getState();
    expect(state.operations).toEqual([]);
    expect(state.selectedOperation).toBeNull();
  });
});

describe("extractGraphqlOperations", () => {
  let extractGraphqlOperations: Awaited<ReturnType<typeof getExtractor>>;

  beforeEach(async () => {
    extractGraphqlOperations = await getExtractor();
  });

  test("기본 query 오퍼레이션 추출", () => {
    const tx = createTransaction();
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(1);
    expect(ops[0].operationType).toBe("query");
    expect(ops[0].operationName).toBe("GetUser");
    expect(ops[0].endpoint).toBe("https://api.example.com/graphql");
    expect(ops[0].responseData).toEqual({ user: { name: "Alice" } });
    expect(ops[0].statusCode).toBe(200);
  });

  test("mutation 오퍼레이션 추출", () => {
    const tx = createTransaction({
      request: {
        method: "POST",
        uri: "https://api.example.com/graphql",
        version: "HTTP/1.1",
        headers: {},
        body: null,
        time: 1700000000000000000,
        id: "tx-2",
        data_type: "GraphQL",
        body_json: { query: "mutation CreateUser { createUser { id } }" },
        body_size: 50,
      },
    });
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(1);
    expect(ops[0].operationType).toBe("mutation");
    expect(ops[0].operationName).toBe("CreateUser");
  });

  test("subscription 오퍼레이션 추출", () => {
    const tx = createTransaction({
      request: {
        method: "POST",
        uri: "https://api.example.com/graphql",
        version: "HTTP/1.1",
        headers: {},
        body: null,
        time: 1700000000000000000,
        id: "tx-3",
        data_type: "GraphQL",
        body_json: { query: "subscription OnMessage { messageAdded { text } }" },
        body_size: 50,
      },
    });
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(1);
    expect(ops[0].operationType).toBe("subscription");
  });

  test("익명 쿼리 (operationName 없음)", () => {
    const tx = createTransaction({
      request: {
        method: "POST",
        uri: "https://api.example.com/graphql",
        version: "HTTP/1.1",
        headers: {},
        body: null,
        time: 1700000000000000000,
        id: "tx-4",
        data_type: "GraphQL",
        body_json: { query: "{ user { name } }" },
        body_size: 30,
      },
    });
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(1);
    expect(ops[0].operationName).toBeNull();
    expect(ops[0].operationType).toBe("query");
  });

  test("data_type이 GraphQL이 아닌 경우 빈 배열 반환", () => {
    const tx = createTransaction({
      request: {
        method: "POST",
        uri: "https://api.example.com/api",
        version: "HTTP/1.1",
        headers: {},
        body: null,
        time: 1700000000000000000,
        id: "tx-5",
        data_type: "JSON",
        body_json: { query: "{ user { name } }" },
        body_size: 30,
      },
    });
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(0);
  });

  test("request가 null인 경우 빈 배열 반환", () => {
    const tx: HttpTransaction = {
      request: null,
      response: null,
    };
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(0);
  });

  test("body_json이 null인 경우 빈 배열 반환", () => {
    const tx = createTransaction({
      request: {
        method: "POST",
        uri: "https://api.example.com/graphql",
        version: "HTTP/1.1",
        headers: {},
        body: null,
        time: 1700000000000000000,
        id: "tx-6",
        data_type: "GraphQL",
        body_json: undefined,
        body_size: 0,
      },
    });
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(0);
  });

  test("배치 쿼리 추출", () => {
    const tx = createTransaction({
      request: {
        method: "POST",
        uri: "https://api.example.com/graphql",
        version: "HTTP/1.1",
        headers: {},
        body: null,
        time: 1700000000000000000,
        id: "tx-7",
        data_type: "GraphQL",
        body_json: [
          { query: "query GetUser { user { name } }", operationName: "GetUser" },
          { query: "mutation DeleteUser { deleteUser(id: 1) }", operationName: "DeleteUser" },
        ],
        body_size: 100,
      },
      response: {
        status: 200,
        version: "HTTP/1.1",
        headers: {},
        body: null,
        time: 1700000000100000000,
        id: "tx-7",
        data_type: "JSON",
        body_json: [
          { data: { user: { name: "Alice" } } },
          { data: { deleteUser: true } },
        ],
        body_size: 80,
      },
    });
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(2);
    expect(ops[0].id).toBe("tx-7-0");
    expect(ops[0].operationType).toBe("query");
    expect(ops[0].batchIndex).toBe(0);
    expect(ops[0].batchSize).toBe(2);
    expect(ops[1].id).toBe("tx-7-1");
    expect(ops[1].operationType).toBe("mutation");
    expect(ops[1].batchIndex).toBe(1);
    expect(ops[1].batchSize).toBe(2);
    expect(ops[0].responseData).toEqual({ user: { name: "Alice" } });
    expect(ops[1].responseData).toEqual({ deleteUser: true });
  });

  test("배치 쿼리 - 단일 항목은 배치로 처리하지 않음", () => {
    const tx = createTransaction({
      request: {
        method: "POST",
        uri: "https://api.example.com/graphql",
        version: "HTTP/1.1",
        headers: {},
        body: null,
        time: 1700000000000000000,
        id: "tx-8",
        data_type: "GraphQL",
        body_json: [{ query: "{ user { name } }" }],
        body_size: 30,
      },
    });
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(1);
    expect(ops[0].batchIndex).toBeNull();
    expect(ops[0].batchSize).toBeNull();
  });

  test("응답에 에러가 있는 경우", () => {
    const tx = createTransaction({
      response: {
        status: 200,
        version: "HTTP/1.1",
        headers: {},
        body: null,
        time: 1700000000100000000,
        id: "tx-1",
        data_type: "JSON",
        body_json: {
          data: null,
          errors: [{ message: "Not found", locations: [{ line: 1, column: 3 }] }],
        },
        body_size: 60,
      },
    });
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(1);
    expect(ops[0].responseErrors).toHaveLength(1);
    expect(ops[0].responseErrors![0].message).toBe("Not found");
  });

  test("응답이 없는 경우 (pending)", () => {
    const tx = createTransaction({
      response: null,
    });
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(1);
    expect(ops[0].responseTime).toBeNull();
    expect(ops[0].responseData).toBeNull();
    expect(ops[0].statusCode).toBeNull();
  });

  test("빈 query string은 필터링", () => {
    const tx = createTransaction({
      request: {
        method: "POST",
        uri: "https://api.example.com/graphql",
        version: "HTTP/1.1",
        headers: {},
        body: null,
        time: 1700000000000000000,
        id: "tx-9",
        data_type: "GraphQL",
        body_json: { query: "" },
        body_size: 10,
      },
    });
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(0);
  });

  test("operationName이 빈 문자열인 경우 null로 처리", () => {
    const tx = createTransaction({
      request: {
        method: "POST",
        uri: "https://api.example.com/graphql",
        version: "HTTP/1.1",
        headers: {},
        body: null,
        time: 1700000000000000000,
        id: "tx-10",
        data_type: "GraphQL",
        body_json: { query: "{ user { name } }", operationName: "" },
        body_size: 30,
      },
    });
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(1);
    expect(ops[0].operationName).toBeNull();
  });

  test("variables 포함", () => {
    const tx = createTransaction();
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(1);
    expect(ops[0].variables).toEqual({ id: "1" });
  });

  test("response extensions 추출", () => {
    const tx = createTransaction({
      response: {
        status: 200,
        version: "HTTP/1.1",
        headers: {},
        body: null,
        time: 1700000000100000000,
        id: "tx-1",
        data_type: "JSON",
        body_json: {
          data: { user: null },
          extensions: { complexity: 42 },
        },
        body_size: 50,
      },
    });
    const ops = extractGraphqlOperations(tx);
    expect(ops).toHaveLength(1);
    expect(ops[0].responseExtensions).toEqual({ complexity: 42 });
  });
});
