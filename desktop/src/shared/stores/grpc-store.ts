import { create } from "zustand";
import type { GrpcCall } from "@/entities/grpc";
import type { HttpTransaction } from "@/entities/proxy";

const GRPC_STATUS_NAMES: Record<number, string> = {
  0: "OK",
  1: "CANCELLED",
  2: "UNKNOWN",
  3: "INVALID_ARGUMENT",
  4: "DEADLINE_EXCEEDED",
  5: "NOT_FOUND",
  6: "ALREADY_EXISTS",
  7: "PERMISSION_DENIED",
  8: "RESOURCE_EXHAUSTED",
  9: "FAILED_PRECONDITION",
  10: "ABORTED",
  11: "OUT_OF_RANGE",
  12: "UNIMPLEMENTED",
  13: "INTERNAL",
  14: "UNAVAILABLE",
  15: "DATA_LOSS",
  16: "UNAUTHENTICATED",
};

interface GrpcStoreState {
  selectedCall: GrpcCall | null;
  setSelectedCall: (call: GrpcCall | null) => void;
  clearAll: () => void;
}

export const useGrpcStore = create<GrpcStoreState>()((set) => ({
  selectedCall: null,
  setSelectedCall: (call) => set({ selectedCall: call }),
  clearAll: () => set({ selectedCall: null }),
}));

/** HttpTransaction에서 gRPC Call을 추출 */
export function extractGrpcCall(tx: HttpTransaction): GrpcCall | null {
  const req = tx.request;
  if (!req) return null;

  // data_type이 Grpc이거나, grpc_metadata가 있는 경우
  const reqMeta = req.grpc_metadata;
  if (req.data_type !== "Grpc" && !reqMeta) return null;

  const resMeta = tx.response?.grpc_metadata;
  const statusCode = resMeta?.status_code ?? null;

  return {
    id: req.id,
    service: reqMeta?.service ?? null,
    method: reqMeta?.method ?? null,
    endpoint: req.uri,
    requestTime: req.time,
    responseTime: tx.response?.time ?? null,
    statusCode,
    statusName: statusCode != null ? (GRPC_STATUS_NAMES[statusCode] ?? "UNKNOWN") : null,
    statusMessage: resMeta?.status_message ?? null,
    contentSubtype: reqMeta?.content_subtype ?? resMeta?.content_subtype ?? null,
    requestFrameCount: reqMeta?.frame_count ?? 0,
    responseFrameCount: resMeta?.frame_count ?? 0,
    streamingType: reqMeta?.streaming_type ?? "Unary",
    httpStatus: tx.response?.status ?? null,
    requestSize: req.body_size,
    responseSize: tx.response?.body_size ?? 0,
  };
}
