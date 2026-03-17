export type GraphqlOperationType = "query" | "mutation" | "subscription";

export interface GraphqlOperation {
  /** 트랜잭션 요청 ID */
  id: string;
  /** 오퍼레이션 타입 */
  operationType: GraphqlOperationType;
  /** 오퍼레이션 이름 */
  operationName: string | null;
  /** GraphQL 쿼리 문자열 */
  query: string;
  /** 변수 (JSON) */
  variables: unknown | null;
  /** 엔드포인트 URI */
  endpoint: string;
  /** 요청 시간 (나노초) */
  requestTime: number;
  /** 응답 시간 (나노초) */
  responseTime: number | null;
  /** 응답 데이터 */
  responseData: unknown | null;
  /** 응답 에러 */
  responseErrors: GraphqlError[] | null;
  /** 응답 extensions */
  responseExtensions: unknown | null;
  /** HTTP 상태 코드 */
  statusCode: number | null;
  /** 배치 인덱스 */
  batchIndex: number | null;
  /** 배치 크기 */
  batchSize: number | null;
}

export interface GraphqlError {
  message: string;
  locations?: { line: number; column: number }[];
  path?: unknown[];
  extensions?: unknown;
}
