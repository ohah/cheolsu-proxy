import { invoke } from "@tauri-apps/api/core";

export interface ProxyStartResult {
  status: boolean;
  message: string;
}

export async function startProxyV2(port: number = 8100): Promise<ProxyStartResult> {
  return invoke("start_proxy_v2", { addr: `127.0.0.1:${port}` });
}

export async function stopProxyV2(): Promise<void> {
  return invoke("stop_proxy_v2");
}

export async function getProxyV2Status(): Promise<boolean> {
  return invoke("proxy_v2_status");
}

export async function cleanOldProxyCache(days: number): Promise<string> {
  return invoke("clean_old_proxy_cache", { days });
}

export async function updateInterceptRules(
  rules: import("@/entities/intercept-rule").InterceptRule[],
): Promise<void> {
  return invoke("update_intercept_rules_v2", { rules });
}

export interface ReplayRequestParams {
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
}

export interface ReplayResponse {
  status: number;
  headers: Record<string, string>;
  body?: string;
  body_size: number;
  elapsed_ms: number;
}

export interface SequenceReplayResult {
  index: number;
  url: string;
  method: string;
  response?: ReplayResponse;
  error?: string;
}

export async function replayRequest(params: ReplayRequestParams): Promise<ReplayResponse> {
  return invoke("replay_request", { params });
}

export interface WsInjectParams {
  connection_id: string;
  direction: "to_client" | "to_server";
  payload: string;
  is_binary: boolean;
}

export async function wsInjectMessage(params: WsInjectParams): Promise<void> {
  return invoke("ws_inject_message", { params });
}

export async function replaySequence(
  requests: ReplayRequestParams[],
): Promise<SequenceReplayResult[]> {
  return invoke("replay_sequence", { requests });
}

export interface ServerReplayEntry {
  id: string;
  method: string;
  url: string;
  status: number;
  headers: Record<string, string>;
  body?: string;
}

export async function updateServerReplay(entries: ServerReplayEntry[]): Promise<void> {
  return invoke("update_server_replay", { entries });
}

export async function getMcpServerPath(): Promise<string> {
  return invoke("get_mcp_server_path");
}

export async function installCli(): Promise<string> {
  return invoke("install_cli");
}

export async function uninstallCli(): Promise<string> {
  return invoke("uninstall_cli");
}

export async function checkCliInstalled(): Promise<boolean> {
  return invoke("check_cli_installed");
}

export async function loadScript(path?: string, code?: string): Promise<void> {
  return invoke("load_script", { path, code });
}

export async function unloadScript(): Promise<void> {
  return invoke("unload_script");
}
