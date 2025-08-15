
// A simplified representation of the data models from proxyapi_models

export interface ProxiedRequest {
  method: string;
  uri: string;
  version: string;
  headers: Record<string, string>;
  body: Uint8Array;
  time: number;
}

export interface ProxiedResponse {
  status: number;
  version: string;
  headers: Record<string, string>;
  body: Uint8Array;
  time: number;
}

export interface RequestInfo {
  request: ProxiedRequest | null;
  response: ProxiedResponse | null;
}
