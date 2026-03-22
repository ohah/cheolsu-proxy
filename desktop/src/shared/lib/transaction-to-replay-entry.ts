import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/entities/proxy";
import type { ServerReplayEntry } from "@/shared/api/proxy";

export function transactionToReplayEntry(tx: HttpTransaction): ServerReplayEntry | null {
  const { request, response } = tx;
  if (!request || !response) return null;

  let body: string | undefined;
  if (response.body_json !== undefined && response.body_json !== null) {
    body =
      typeof response.body_json === "string"
        ? response.body_json
        : JSON.stringify(response.body_json);
  } else if (response.body && response.data_type && isTextBasedDataType(response.data_type)) {
    try {
      body = new TextDecoder().decode(response.body);
    } catch {
      body = undefined;
    }
  }

  return {
    id: request.id,
    method: request.method,
    url: request.uri,
    status: response.status,
    headers: response.headers || {},
    body,
  };
}
