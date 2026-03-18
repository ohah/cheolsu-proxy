import { invoke } from "@tauri-apps/api/core";

export async function updateInterceptRules(
  rules: import("@/entities/intercept-rule").InterceptRule[],
): Promise<void> {
  return invoke("update_intercept_rules_v2", { rules });
}

// ─── Breakpoint ──────────────────────────────────────────

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

// ─── Reverse Proxy ───────────────────────────────────────

export interface ReverseProxyRule {
  id: string;
  match_host: string;
  backend_scheme: string;
  backend_host: string;
  backend_port: number;
  rewrite_host: boolean;
  enabled: boolean;
}

export async function updateReverseProxyRules(rules: ReverseProxyRule[]): Promise<void> {
  return invoke("update_reverse_proxy_rules", { rules });
}

// ─── SSL Proxying ────────────────────────────────────────

export type SslProxyingMode = "blacklist" | "whitelist";

export interface SslProxyingEntry {
  pattern: string;
  enabled: boolean;
}

export async function updateSslProxyingList(
  mode: SslProxyingMode,
  entries: SslProxyingEntry[],
): Promise<void> {
  return invoke("update_ssl_proxying_list", { mode, entries });
}

export async function updateDefaultPassthroughDomains(entries: SslProxyingEntry[]): Promise<void> {
  return invoke("update_default_passthrough_domains", { entries });
}

/** Rust 백엔드에서 기본 패스스루 도메인 목록을 가져옵니다 (Single Source of Truth) */
export async function getDefaultPassthroughDomains(): Promise<SslProxyingEntry[]> {
  return invoke("get_default_passthrough_domains");
}

// ─── Never Passthrough ────────────────────────────────

export async function updateNeverPassthroughDomains(entries: string[]): Promise<void> {
  return invoke("update_never_passthrough_domains", { entries });
}

export async function getNeverPassthroughDomains(): Promise<string[]> {
  return invoke("get_never_passthrough_domains");
}
