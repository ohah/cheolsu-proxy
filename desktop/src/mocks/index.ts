import type { HttpTransaction } from "@/entities/proxy";

import { MOCK_ERROR_TRANSACTIONS } from "./mock-errors";
import { MOCK_GRAPHQL_TRANSACTIONS } from "./mock-graphql";
import { MOCK_MISC_TRANSACTIONS } from "./mock-misc";
import { MOCK_REST_API_TRANSACTIONS } from "./mock-rest-api";
import { MOCK_STATIC_ASSET_TRANSACTIONS } from "./mock-static-assets";
import { MOCK_WEBSOCKET_TRANSACTIONS } from "./mock-websocket";

export { MOCK_ERROR_TRANSACTIONS } from "./mock-errors";
export { MOCK_GRAPHQL_TRANSACTIONS } from "./mock-graphql";
export { MOCK_MISC_TRANSACTIONS } from "./mock-misc";
export { MOCK_REST_API_TRANSACTIONS } from "./mock-rest-api";
export { MOCK_STATIC_ASSET_TRANSACTIONS } from "./mock-static-assets";
export { MOCK_WEBSOCKET_TRANSACTIONS } from "./mock-websocket";
export { jsonBody, makeRequest, makeResponse, textBody, tx } from "./mock-helpers";

/** 모든 도메인의 목 트랜잭션을 결합한 배열 (기존 MOCK_TRANSACTIONS와 동일) */
export const MOCK_TRANSACTIONS: HttpTransaction[] = [
  ...MOCK_REST_API_TRANSACTIONS,
  ...MOCK_STATIC_ASSET_TRANSACTIONS,
  ...MOCK_ERROR_TRANSACTIONS,
  ...MOCK_GRAPHQL_TRANSACTIONS,
  ...MOCK_MISC_TRANSACTIONS,
  ...MOCK_WEBSOCKET_TRANSACTIONS,
];
