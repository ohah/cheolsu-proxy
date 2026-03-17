export interface GrpcCall {
  /** 트랜잭션 요청 ID */
  id: string;
  /** gRPC 서비스 이름 (예: package.ServiceName) */
  service: string | null;
  /** gRPC 메서드 이름 */
  method: string | null;
  /** 엔드포인트 URI */
  endpoint: string;
  /** 요청 시간 (나노초) */
  requestTime: number;
  /** 응답 시간 (나노초) */
  responseTime: number | null;
  /** gRPC 상태 코드 */
  statusCode: number | null;
  /** gRPC 상태 이름 (OK, INTERNAL 등) */
  statusName: string | null;
  /** gRPC 상태 메시지 */
  statusMessage: string | null;
  /** 콘텐츠 서브타입 (proto, json) */
  contentSubtype: string | null;
  /** 요청 프레임 수 */
  requestFrameCount: number;
  /** 응답 프레임 수 */
  responseFrameCount: number;
  /** HTTP 상태 코드 */
  httpStatus: number | null;
  /** 요청 바디 크기 */
  requestSize: number;
  /** 응답 바디 크기 */
  responseSize: number;
}
