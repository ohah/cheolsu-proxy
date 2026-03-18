import type { DataType, HttpRequest, HttpResponse, HttpTransaction } from "@/entities/proxy";

let idCounter = 0;
function nextId(): string {
  idCounter += 1;
  return `mock-${idCounter}-${Date.now()}`;
}

export function makeRequest(
  overrides: Partial<HttpRequest> & { method: string; uri: string },
): HttpRequest {
  const id = overrides.id ?? nextId();
  return {
    method: overrides.method,
    uri: overrides.uri,
    version: overrides.version ?? "HTTP/2.0",
    headers: overrides.headers ?? { "content-type": "application/json" },
    body: overrides.body ?? null,
    time: overrides.time ?? Date.now(),
    id,
    data_type: overrides.data_type ?? "Json",
    body_json: overrides.body_json,
    body_size: overrides.body_size ?? 0,
  };
}

export function makeResponse(
  id: string,
  overrides: Partial<HttpResponse> & { status: number },
): HttpResponse {
  return {
    status: overrides.status,
    version: overrides.version ?? "HTTP/2.0",
    headers: overrides.headers ?? { "content-type": "application/json" },
    body: overrides.body ?? null,
    time: overrides.time ?? Date.now() + 50,
    id,
    data_type: overrides.data_type ?? "Json",
    body_json: overrides.body_json,
    body_size: overrides.body_size ?? 0,
  };
}

export function tx(
  req: Partial<HttpRequest> & { method: string; uri: string },
  res?: Partial<HttpResponse> & { status: number },
): HttpTransaction {
  const request = makeRequest(req);
  const response = res ? makeResponse(request.id, res) : null;
  return { request, response };
}

export const jsonBody = (obj: any): { body_json: any; body_size: number; data_type: DataType } => ({
  body_json: obj,
  body_size: JSON.stringify(obj).length,
  data_type: "Json",
});

export const textBody = (
  text: string,
  dataType: DataType = "Text",
): { body: Uint8Array; body_size: number; data_type: DataType } => ({
  body: new TextEncoder().encode(text),
  body_size: text.length,
  data_type: dataType,
});
