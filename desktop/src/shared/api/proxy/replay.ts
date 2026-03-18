import { invoke } from "@tauri-apps/api/core";

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

export async function replaySequence(
  requests: ReplayRequestParams[],
): Promise<SequenceReplayResult[]> {
  return invoke("replay_sequence", { requests });
}

// ─── Advanced Repeat ─────────────────────────────────────

export interface AdvancedRepeatParams {
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
  iterations: number;
  concurrency: number;
  delay_ms: number;
}

export interface AdvancedRepeatProgress {
  completed: number;
  total: number;
  success_count: number;
  failure_count: number;
  last_status?: number;
  last_elapsed_ms?: number;
}

export interface AdvancedRepeatResult {
  total: number;
  success_count: number;
  failure_count: number;
  min_time_ms: number;
  max_time_ms: number;
  avg_time_ms: number;
  total_time_ms: number;
  requests_per_second: number;
  status_codes: Record<number, number>;
}

export async function advancedRepeat(params: AdvancedRepeatParams): Promise<AdvancedRepeatResult> {
  return invoke("advanced_repeat", { params });
}

// ─── Server Replay ───────────────────────────────────────

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

// ─── WebSocket Inject ────────────────────────────────────

export interface WsInjectParams {
  connection_id: string;
  direction: "to_client" | "to_server";
  payload: string;
  is_binary: boolean;
}

export async function wsInjectMessage(params: WsInjectParams): Promise<void> {
  return invoke("ws_inject_message", { params });
}
