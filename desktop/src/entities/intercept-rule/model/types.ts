export type InterceptActionType =
  | "block"
  | "modify_request"
  | "modify_response"
  | "map_local"
  | "map_remote"
  | "rewrite"
  | "throttle";

export interface BlockAction {
  type: "block";
  status_code: number;
  body: string;
}

export interface ModifyRequestAction {
  type: "modify_request";
  add_headers: Record<string, string>;
  remove_headers: string[];
  set_body: string | null;
}

export interface ModifyResponseAction {
  type: "modify_response";
  set_status: number | null;
  add_headers: Record<string, string>;
  remove_headers: string[];
  set_body: string | null;
}

export interface MapLocalAction {
  type: "map_local";
  file_path: string;
  status_code: number;
  headers: Record<string, string>;
}

export interface MapRemoteAction {
  type: "map_remote";
  target_url: string;
  preserve_path: boolean;
}

export type RewriteTarget = "request_header" | "response_header" | "request_body" | "response_body";

export interface RewriteAction {
  type: "rewrite";
  target: RewriteTarget;
  match_pattern: string;
  replace_with: string;
}

export interface ThrottleAction {
  type: "throttle";
  download_rate: number | null;
  upload_rate: number | null;
  latency_ms: number;
}

export type InterceptAction =
  | BlockAction
  | ModifyRequestAction
  | ModifyResponseAction
  | MapLocalAction
  | MapRemoteAction
  | RewriteAction
  | ThrottleAction;

export interface InterceptRule {
  id: string;
  name: string;
  enabled: boolean;
  pattern: string;
  method: string | null;
  action: InterceptAction;
}
