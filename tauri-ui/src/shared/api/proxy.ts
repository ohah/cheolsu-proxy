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
