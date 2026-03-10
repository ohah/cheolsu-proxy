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

export async function getCaCertPath(): Promise<string> {
  return invoke("get_ca_cert_path");
}

export async function checkCaInstalled(): Promise<boolean> {
  return invoke("check_ca_installed");
}

export async function installCaCert(): Promise<string> {
  return invoke("install_ca_cert");
}

export async function uninstallCaCert(): Promise<string> {
  return invoke("uninstall_ca_cert");
}

export interface ThrottleConfig {
  enabled: boolean;
  download_rate: number | null;
  upload_rate: number | null;
  latency_ms: number;
}

export async function updateThrottle(config: ThrottleConfig | null): Promise<void> {
  return invoke("update_throttle", { config });
}

export async function loadScript(path?: string, code?: string): Promise<void> {
  return invoke("load_script", { path, code });
}

export async function unloadScript(): Promise<void> {
  return invoke("unload_script");
}

export interface CertDownloadInfo {
  port: number;
  local_ips: string[];
  download_url: string;
  direct_url: string;
  qr_code_base64: string;
}

export async function getCertDownloadInfo(port: number): Promise<CertDownloadInfo> {
  return invoke("get_cert_download_info", { port });
}

// ─── Traffic Diff ────────────────────────────────────────

export interface DiffTransactionData {
  method?: string;
  uri?: string;
  status?: number;
  headers: [string, string][];
  body?: string;
  body_size: number;
  data_type?: string;
}

export interface DiffTransactionPair {
  request?: DiffTransactionData;
  response?: DiffTransactionData;
}

export interface HeaderDiff {
  type: "added" | "removed" | "modified";
  key: string;
  value?: string;
  old_value?: string;
  new_value?: string;
}

export interface DiffLine {
  line_number: number;
  content: string;
}

export interface JsonDiffEntry {
  path: string;
  change_type: string;
  old_value?: string;
  new_value?: string;
}

export type BodyDiff =
  | { type: "text"; additions: DiffLine[]; deletions: DiffLine[]; unchanged: number }
  | { type: "json"; changes: JsonDiffEntry[] }
  | { type: "binary"; old_size: number; new_size: number };

export interface TransactionPartDiff {
  method_diff?: [string, string];
  url_diff?: [string, string];
  status_diff?: [number, number];
  header_diffs: HeaderDiff[];
  body_diff?: BodyDiff;
}

export interface TrafficDiff {
  request_diff?: TransactionPartDiff;
  response_diff?: TransactionPartDiff;
}

export async function diffTransactions(
  transactionA: DiffTransactionData,
  transactionB: DiffTransactionData,
): Promise<TrafficDiff> {
  return invoke("diff_transactions", { transactionA, transactionB });
}

export async function diffTransactionPairs(
  pairA: DiffTransactionPair,
  pairB: DiffTransactionPair,
): Promise<TrafficDiff> {
  return invoke("diff_transaction_pairs", { pairA, pairB });
}

export async function updateBreakpointRules(
  rules: import("@/entities/breakpoint").BreakpointRule[],
): Promise<void> {
  return invoke("update_breakpoint_rules", { rules });
}

export async function resolveBreakpoint(
  id: string,
  action: import("@/entities/breakpoint").BreakpointAction,
): Promise<void> {
  return invoke("resolve_breakpoint", { id, action });
}

// --- Session save/load ---

export async function saveSession(path: string, transactionsJson: string): Promise<void> {
  return invoke("save_session", { path, transactionsJson });
}

export interface LoadSessionResult {
  name: string | null;
  description: string | null;
  transaction_count: number;
  transactions_json: string;
}

export async function loadSession(path: string): Promise<LoadSessionResult> {
  return invoke("load_session", { path });
}

export async function importHarFile(path: string): Promise<string> {
  return invoke("import_har_file_cmd", { path });
}

// --- Auto session save/load ---

export async function autosaveSession(transactionsJson: string): Promise<void> {
  return invoke("autosave_session", { transactionsJson });
}

export async function autoloadSession(): Promise<LoadSessionResult | null> {
  return invoke("autoload_session");
}

// ─── Host Mapping ────────────────────────────────────────

export interface HostMapping {
  id: string;
  source_host: string;
  source_port: number | null;
  target_host: string;
  target_port: number | null;
  enabled: boolean;
}

export async function updateHostMappings(mappings: HostMapping[]): Promise<void> {
  return invoke("update_host_mappings", { mappings });
}

// ─── SSL Proxying ────────────────────────────────────────

export interface SslProxyingEntry {
  pattern: string;
  enabled: boolean;
}

export async function updateSslProxyingList(entries: SslProxyingEntry[]): Promise<void> {
  return invoke("update_ssl_proxying_list", { entries });
}

// ─── Proxy Authentication ────────────────────────────────

export interface ProxyAuthConfig {
  enabled: boolean;
  username: string;
  password: string;
}

export async function updateProxyAuth(config: ProxyAuthConfig): Promise<void> {
  return invoke("update_proxy_auth", { config });
}

// ─── Quick Settings ──────────────────────────────────────

export async function updateQuickSettings(
  noCaching: boolean,
  blockCookies: boolean,
  noGzip: boolean,
): Promise<void> {
  return invoke("update_quick_settings", { noCaching, blockCookies, noGzip });
}
