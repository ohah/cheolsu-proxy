import { invoke } from "@tauri-apps/api/core";

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

// ─── Certificate Info ────────────────────────────────────

export interface CertificateInfo {
  subject_cn: string | null;
  issuer_cn: string | null;
  organization: string | null;
  sans_dns: string[];
  sans_ip: string[];
  not_before: string;
  not_after: string;
  serial_number: string;
  fingerprint_sha256: string;
  is_ca: boolean;
  chain_length: number;
}

export async function parseCertificateInfo(certPath: string): Promise<CertificateInfo> {
  return invoke("parse_certificate_info", { certPath });
}

// ─── Client Certificate (mTLS) ───────────────────────────

export interface DomainClientCertConfig {
  domain_pattern: string;
  cert_path: string;
  key_path: string;
  enabled: boolean;
}

export interface ClientCertConfig {
  cert_path: string;
  key_path: string;
  enabled: boolean;
  domain_certs?: DomainClientCertConfig[];
}

export async function updateClientCertificate(config: ClientCertConfig | null): Promise<void> {
  return invoke("update_client_certificate", { config });
}

// ─── Request Client Certificate (Proxy → Client) ────────

export interface RequestClientCertConfig {
  enabled: boolean;
  ca_cert_path?: string | null;
  required: boolean;
}

export async function updateRequestClientCert(
  config: RequestClientCertConfig | null,
): Promise<void> {
  return invoke("update_request_client_cert", { config });
}

// ─── Custom CA Certificate ───────────────────────────────

export async function importCustomCa(certPath: string, keyPath: string): Promise<CertificateInfo> {
  return invoke("import_custom_ca", { certPath, keyPath });
}

export async function importCustomCaPkcs12(
  p12Path: string,
  password: string,
): Promise<CertificateInfo> {
  return invoke("import_custom_ca_pkcs12", { p12Path, password });
}

export async function removeCustomCa(): Promise<void> {
  return invoke("remove_custom_ca");
}

export async function getCustomCaStatus(): Promise<CertificateInfo | null> {
  return invoke("get_custom_ca_status");
}
