import { invoke } from '@tauri-apps/api/core';

export async function fetchProxyStatus(): Promise<boolean> {
  return await invoke('proxy_status');
}

export async function startProxy(address: string): Promise<void> {
  console.log('startProxy!!');
  return await invoke('start_proxy', { addr: address });
}

export async function stopProxy(): Promise<void> {
  return await invoke('stop_proxy');
}

// proxyapi_v2를 사용하는 새로운 프록시 함수들
export interface ProxyStartResult {
  status: boolean;
  message: string;
}

export async function startProxyV2(port: number = 8100): Promise<ProxyStartResult> {
  return invoke('start_proxy_v2', { addr: `127.0.0.1:${port}` });
}

export async function stopProxyV2(): Promise<void> {
  return invoke('stop_proxy_v2');
}

export async function getProxyV2Status(): Promise<boolean> {
  return invoke('proxy_v2_status');
}
