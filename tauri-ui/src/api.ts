import { invoke } from '@tauri-apps/api/core';

export async function fetchProxyStatus(): Promise<boolean> {
  return await invoke('proxy_status');
}

export async function startProxy(address: string): Promise<void> {
  return await invoke('start_proxy', { addr: address });
}

export async function stopProxy(): Promise<void> {
  return await invoke('stop_proxy');
}
