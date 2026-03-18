import { invoke } from "@tauri-apps/api/core";

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
