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

// ─── Proxy Authentication ────────────────────────────────

export interface ProxyAuthConfig {
  enabled: boolean;
  username: string;
  password: string;
}

export async function updateProxyAuth(config: ProxyAuthConfig): Promise<void> {
  return invoke("update_proxy_auth", { config });
}

// ─── Connection Strategy ─────────────────────────────────

export type ConnectionStrategy = "lazy" | "eager" | "eager_with_fallback";

export async function updateConnectionStrategy(strategy: ConnectionStrategy): Promise<void> {
  return invoke("update_connection_strategy", { strategy });
}

// ─── Quick Settings ──────────────────────────────────────

export async function updateQuickSettings(
  noCaching: boolean,
  blockCookies: boolean,
  noGzip: boolean,
  blockQuic: boolean,
): Promise<void> {
  return invoke("update_quick_settings", { noCaching, blockCookies, noGzip, blockQuic });
}

// ─── Throttle ────────────────────────────────────────────

export interface ThrottleConfig {
  enabled: boolean;
  download_rate: number | null;
  upload_rate: number | null;
  latency_ms: number;
}

export async function updateThrottle(config: ThrottleConfig | null): Promise<void> {
  return invoke("update_throttle", { config });
}
