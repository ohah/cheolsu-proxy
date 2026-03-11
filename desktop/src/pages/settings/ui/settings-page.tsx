import { useState, useEffect, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { useProxyStore } from "@/shared/stores/proxy-store";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import { useSslProxyingStore } from "@/shared/stores/ssl-proxying-store";
import {
  updateProxyAuth,
  updateClientCertificate,
  updateRequestClientCert,
  parseCertificateInfo,
  type ProxyAuthConfig,
  type CertificateInfo,
  type DomainClientCertConfig,
  type RequestClientCertConfig,
} from "@/shared/api/proxy";
import { Button, Input, Switch, Badge } from "@/shared/ui";
import {
  GeneralSettings,
  ProxySettings,
  ThrottleSettings,
  CertificateSettings,
  ShortcutSettings,
  CliSettings,
} from "./components";

export function SettingsPage() {
  return (
    <div className="flex-1 flex flex-col h-full overflow-auto">
      <div className="p-6 space-y-6">
        <div>
          <h1 className="text-2xl font-bold text-foreground">
            <Trans>Settings</Trans>
          </h1>
          <p className="text-muted-foreground">
            <Trans>Proxy configuration and preferences</Trans>
          </p>
        </div>

        <GeneralSettings />
        <CertificateSettings />
        <CliSettings />
        <ShortcutSettings />
        <ThrottleSettings />
        <SslProxyingSection />
        <ProxyAuthSection />
        <ProxySettings />
        <ClientCertificateSection />
        <RequestClientCertSection />
      </div>
    </div>
  );
}

function ClientCertificateSection() {
  const { t } = useLingui();
  const [certEnabled, setCertEnabled] = useState(false);
  const [certPath, setCertPath] = useState("");
  const [keyPath, setKeyPath] = useState("");
  const [certSaving, setCertSaving] = useState(false);
  const [certStatus, setCertStatus] = useState<"idle" | "saved" | "error">("idle");
  const [certInfo, setCertInfo] = useState<CertificateInfo | null>(null);
  const [certInfoLoading, setCertInfoLoading] = useState(false);

  // 도메인별 인증서 상태
  const [domainCerts, setDomainCerts] = useState<DomainClientCertConfig[]>([]);
  const [newDomainPattern, setNewDomainPattern] = useState("");
  const [newDomainCertPath, setNewDomainCertPath] = useState("");
  const [newDomainKeyPath, setNewDomainKeyPath] = useState("");

  const loadCertInfo = useCallback(async (path: string) => {
    setCertInfoLoading(true);
    try {
      const info = await parseCertificateInfo(path);
      setCertInfo(info);
    } catch {
      setCertInfo(null);
    } finally {
      setCertInfoLoading(false);
    }
  }, []);

  const handleSelectCert = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Certificate", extensions: ["pem", "crt", "cer"] }],
    });
    if (selected) {
      const path = selected as string;
      setCertPath(path);
      setCertStatus("idle");
      loadCertInfo(path);
    }
  }, [loadCertInfo]);

  const handleSelectKey = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Key", extensions: ["pem", "key"] }],
    });
    if (selected) {
      setKeyPath(selected as string);
      setCertStatus("idle");
    }
  }, []);

  // 도메인별 인증서 파일 선택
  const handleSelectDomainCert = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Certificate", extensions: ["pem", "crt", "cer"] }],
    });
    if (selected) {
      setNewDomainCertPath(selected as string);
    }
  }, []);

  const handleSelectDomainKey = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Key", extensions: ["pem", "key"] }],
    });
    if (selected) {
      setNewDomainKeyPath(selected as string);
    }
  }, []);

  const handleAddDomainCert = useCallback(() => {
    const pattern = newDomainPattern.trim();
    if (!pattern || !newDomainCertPath || !newDomainKeyPath) return;
    // 중복 체크
    if (domainCerts.some((dc) => dc.domain_pattern === pattern)) return;
    setDomainCerts((prev) => [
      ...prev,
      {
        domain_pattern: pattern,
        cert_path: newDomainCertPath,
        key_path: newDomainKeyPath,
        enabled: true,
      },
    ]);
    setNewDomainPattern("");
    setNewDomainCertPath("");
    setNewDomainKeyPath("");
    setCertStatus("idle");
  }, [newDomainPattern, newDomainCertPath, newDomainKeyPath, domainCerts]);

  const toggleDomainCert = useCallback((idx: number) => {
    setDomainCerts((prev) =>
      prev.map((dc, i) => (i === idx ? { ...dc, enabled: !dc.enabled } : dc)),
    );
    setCertStatus("idle");
  }, []);

  const removeDomainCert = useCallback((idx: number) => {
    setDomainCerts((prev) => prev.filter((_, i) => i !== idx));
    setCertStatus("idle");
  }, []);

  const handleCertSave = useCallback(async () => {
    setCertSaving(true);
    setCertStatus("idle");
    try {
      if (certEnabled && certPath && keyPath) {
        await updateClientCertificate({
          cert_path: certPath,
          key_path: keyPath,
          enabled: true,
          domain_certs: domainCerts,
        });
      } else {
        await updateClientCertificate(
          certEnabled
            ? { cert_path: certPath, key_path: keyPath, enabled: false, domain_certs: domainCerts }
            : null,
        );
      }
      setCertStatus("saved");
    } catch {
      setCertStatus("error");
    } finally {
      setCertSaving(false);
    }
  }, [certEnabled, certPath, keyPath, domainCerts]);

  return (
    <div className="border rounded-lg p-5 space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Client Certificate</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>
              Present a client certificate when connecting to servers that require mTLS
              authentication
            </Trans>
          </p>
        </div>
        <Switch checked={certEnabled} onCheckedChange={setCertEnabled} />
      </div>

      {certEnabled && (
        <div className="space-y-4 pt-2">
          <div>
            <label className="text-sm font-medium mb-1.5 block">
              <Trans>Certificate File</Trans>
            </label>
            <div className="flex gap-2">
              <Input
                readOnly
                placeholder={t`Select certificate file (.pem, .crt)`}
                value={certPath}
                className="flex-1"
              />
              <Button variant="outline" onClick={handleSelectCert}>
                {t`Browse`}
              </Button>
            </div>
          </div>

          <div>
            <label className="text-sm font-medium mb-1.5 block">
              <Trans>Key File</Trans>
            </label>
            <div className="flex gap-2">
              <Input
                readOnly
                placeholder={t`Select key file (.pem, .key)`}
                value={keyPath}
                className="flex-1"
              />
              <Button variant="outline" onClick={handleSelectKey}>
                {t`Browse`}
              </Button>
            </div>
          </div>

          {certInfoLoading && (
            <p className="text-sm text-muted-foreground">
              <Trans>Loading certificate info...</Trans>
            </p>
          )}

          {certInfo && !certInfoLoading && (
            <div className="bg-muted/50 rounded-lg p-4 space-y-3">
              <h3 className="text-sm font-semibold">
                <Trans>Certificate Details</Trans>
              </h3>
              <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-sm">
                {certInfo.subject_cn && (
                  <>
                    <span className="text-muted-foreground">CN</span>
                    <span className="font-mono">{certInfo.subject_cn}</span>
                  </>
                )}
                {certInfo.issuer_cn && (
                  <>
                    <span className="text-muted-foreground">
                      <Trans>Issuer</Trans>
                    </span>
                    <span className="font-mono">{certInfo.issuer_cn}</span>
                  </>
                )}
                {certInfo.organization && (
                  <>
                    <span className="text-muted-foreground">
                      <Trans>Organization</Trans>
                    </span>
                    <span className="font-mono">{certInfo.organization}</span>
                  </>
                )}
                {certInfo.sans_dns.length > 0 && (
                  <>
                    <span className="text-muted-foreground">SAN (DNS)</span>
                    <span className="font-mono">{certInfo.sans_dns.join(", ")}</span>
                  </>
                )}
                {certInfo.sans_ip.length > 0 && (
                  <>
                    <span className="text-muted-foreground">SAN (IP)</span>
                    <span className="font-mono">{certInfo.sans_ip.join(", ")}</span>
                  </>
                )}
                <>
                  <span className="text-muted-foreground">
                    <Trans>Valid From</Trans>
                  </span>
                  <span className="font-mono">{certInfo.not_before}</span>
                </>
                <>
                  <span className="text-muted-foreground">
                    <Trans>Valid Until</Trans>
                  </span>
                  <span className="font-mono">{certInfo.not_after}</span>
                </>
                <>
                  <span className="text-muted-foreground">
                    <Trans>Serial Number</Trans>
                  </span>
                  <span className="font-mono text-xs break-all">{certInfo.serial_number}</span>
                </>
                <>
                  <span className="text-muted-foreground">SHA-256</span>
                  <span className="font-mono text-xs break-all">{certInfo.fingerprint_sha256}</span>
                </>
                <>
                  <span className="text-muted-foreground">CA</span>
                  <span className="font-mono">{certInfo.is_ca ? "Yes" : "No"}</span>
                </>
                <>
                  <span className="text-muted-foreground">
                    <Trans>Chain Length</Trans>
                  </span>
                  <span className="font-mono">{certInfo.chain_length}</span>
                </>
              </div>
              {new Date(certInfo.not_after) < new Date() && (
                <Badge variant="outline" className="text-red-600 border-red-600">
                  <Trans>Certificate expired</Trans>
                </Badge>
              )}
            </div>
          )}

          <p className="text-xs text-muted-foreground">
            <Trans>Supports PEM-encoded certificates and keys (RSA, ECDSA, PKCS#8)</Trans>
          </p>

          {/* 도메인별 인증서 섹션 */}
          <div className="space-y-4 pt-4 border-t">
            <div>
              <h3 className="text-sm font-semibold">
                <Trans>Domain-specific Certificates</Trans>
              </h3>
              <p className="text-xs text-muted-foreground mt-1">
                <Trans>
                  Use different client certificates for specific domains. Supports wildcards
                  (*.example.com). Currently validates certificates only — domain-specific routing
                  coming soon.
                </Trans>
              </p>
            </div>

            {/* 새 도메인 인증서 추가 폼 */}
            <div className="space-y-2">
              <Input
                placeholder="*.example.com"
                value={newDomainPattern}
                onChange={(e) => setNewDomainPattern(e.target.value)}
              />
              <div className="flex gap-2">
                <Input
                  readOnly
                  placeholder={t`Certificate file`}
                  value={newDomainCertPath}
                  className="flex-1"
                />
                <Button variant="outline" onClick={handleSelectDomainCert}>
                  {t`Browse`}
                </Button>
              </div>
              <div className="flex gap-2">
                <Input
                  readOnly
                  placeholder={t`Key file`}
                  value={newDomainKeyPath}
                  className="flex-1"
                />
                <Button variant="outline" onClick={handleSelectDomainKey}>
                  {t`Browse`}
                </Button>
              </div>
              <Button
                onClick={handleAddDomainCert}
                disabled={!newDomainPattern.trim() || !newDomainCertPath || !newDomainKeyPath}
              >
                <Trans>Add Domain Certificate</Trans>
              </Button>
            </div>

            {/* 도메인 인증서 목록 */}
            {domainCerts.length > 0 && (
              <div className="border rounded-lg divide-y">
                {domainCerts.map((dc, idx) => (
                  <div
                    key={dc.domain_pattern}
                    className="flex items-center justify-between px-4 py-2"
                  >
                    <div className="flex items-center gap-3">
                      <Switch checked={dc.enabled} onCheckedChange={() => toggleDomainCert(idx)} />
                      <div>
                        <span
                          className={`font-mono text-sm ${dc.enabled ? "text-foreground" : "text-muted-foreground line-through"}`}
                        >
                          {dc.domain_pattern}
                        </span>
                        <span className="text-xs text-muted-foreground block truncate max-w-xs">
                          {dc.cert_path}
                        </span>
                      </div>
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => removeDomainCert(idx)}
                      className="text-muted-foreground hover:text-destructive"
                    >
                      <Trans>Remove</Trans>
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      <div className="flex items-center gap-3 pt-2">
        <Button onClick={handleCertSave} disabled={certSaving}>
          {certSaving ? t`Saving...` : t`Save`}
        </Button>
        {certStatus === "saved" && (
          <Badge variant="outline" className="text-green-600 border-green-600">
            <Trans>Saved</Trans>
          </Badge>
        )}
        {certStatus === "error" && (
          <Badge variant="outline" className="text-red-600 border-red-600">
            <Trans>Failed — check file paths and proxy status</Trans>
          </Badge>
        )}
      </div>
    </div>
  );
}

function SslProxyingSection() {
  const { t } = useLingui();
  const entries = useSslProxyingStore((s) => s.entries);
  const addEntry = useSslProxyingStore((s) => s.addEntry);
  const removeEntry = useSslProxyingStore((s) => s.removeEntry);
  const toggleEntry = useSslProxyingStore((s) => s.toggleEntry);
  const [newPattern, setNewPattern] = useState("");

  const handleAdd = useCallback(() => {
    const pattern = newPattern.trim();
    if (!pattern) return;
    // 중복 체크
    if (entries.some((e) => e.pattern === pattern)) return;
    addEntry({ pattern, enabled: true });
    setNewPattern("");
  }, [newPattern, entries, addEntry]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAdd();
      }
    },
    [handleAdd],
  );

  const enabledCount = entries.filter((e) => e.enabled).length;

  return (
    <div className="border rounded-lg p-5 space-y-4">
      <div>
        <h2 className="text-lg font-semibold">
          <Trans>SSL Proxying</Trans>
        </h2>
        <p className="text-sm text-muted-foreground">
          {enabledCount === 0 ? (
            <Trans>All HTTPS traffic is being intercepted (no whitelist configured)</Trans>
          ) : (
            <Trans>
              Only whitelisted domains ({enabledCount}) will have HTTPS traffic intercepted
            </Trans>
          )}
        </p>
      </div>

      {/* 도메인 입력 */}
      <div className="flex items-center gap-2">
        <Input
          placeholder={t`example.com, *.example.com, or example.com:443`}
          value={newPattern}
          onChange={(e) => setNewPattern(e.target.value)}
          onKeyDown={handleKeyDown}
          className="flex-1"
        />
        <Button onClick={handleAdd} disabled={!newPattern.trim()}>
          <Trans>Add</Trans>
        </Button>
      </div>

      <p className="text-xs text-muted-foreground">
        <Trans>
          Supports exact domains (example.com), wildcards (*.example.com), and port-specific
          patterns (example.com:443). When the list is empty, all domains are intercepted.
        </Trans>
      </p>

      {/* 도메인 목록 */}
      {entries.length > 0 && (
        <div className="border rounded-lg divide-y">
          {entries.map((entry) => (
            <div key={entry.pattern} className="flex items-center justify-between px-4 py-2">
              <div className="flex items-center gap-3">
                <Switch
                  checked={entry.enabled}
                  onCheckedChange={() => toggleEntry(entry.pattern)}
                />
                <span
                  className={`font-mono text-sm ${entry.enabled ? "text-foreground" : "text-muted-foreground line-through"}`}
                >
                  {entry.pattern}
                </span>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => removeEntry(entry.pattern)}
                className="text-muted-foreground hover:text-destructive"
              >
                <Trans>Remove</Trans>
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function RequestClientCertSection() {
  const { t } = useLingui();
  const [enabled, setEnabled] = useState(false);
  const [caCertPath, setCaCertPath] = useState("");
  const [required, setRequired] = useState(false);
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<"idle" | "saved" | "error">("idle");

  const handleSelectCaCert = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Certificate", extensions: ["pem", "crt", "cer"] }],
    });
    if (selected) {
      setCaCertPath(selected as string);
      setStatus("idle");
    }
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setStatus("idle");
    try {
      if (enabled) {
        const config: RequestClientCertConfig = {
          enabled: true,
          ca_cert_path: caCertPath || null,
          required,
        };
        await updateRequestClientCert(config);
      } else {
        await updateRequestClientCert(null);
      }
      setStatus("saved");
      setTimeout(() => setStatus("idle"), 2000);
    } catch {
      setStatus("error");
    } finally {
      setSaving(false);
    }
  }, [enabled, caCertPath, required]);

  return (
    <div className="border rounded-lg p-5 space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Request Client Certificate</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>Request a client certificate from connecting clients (mTLS server-side)</Trans>
          </p>
        </div>
        <Switch checked={enabled} onCheckedChange={setEnabled} />
      </div>

      {enabled && (
        <div className="space-y-4 pt-2">
          <div>
            <label className="text-sm font-medium mb-1.5 block">
              <Trans>CA Certificate File (optional)</Trans>
            </label>
            <div className="flex gap-2">
              <Input
                readOnly
                placeholder={t`Select CA certificate to verify clients (.pem, .crt)`}
                value={caCertPath}
                className="flex-1"
              />
              <Button variant="outline" onClick={handleSelectCaCert}>
                {t`Browse`}
              </Button>
              {caCertPath && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    setCaCertPath("");
                    setStatus("idle");
                  }}
                >
                  <Trans>Clear</Trans>
                </Button>
              )}
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              <Trans>
                If not specified, any client certificate will be accepted without verification
              </Trans>
            </p>
          </div>

          <div className="flex items-center gap-3">
            <Switch checked={required} onCheckedChange={setRequired} />
            <div>
              <span className="text-sm font-medium">
                <Trans>Require certificate</Trans>
              </span>
              <p className="text-xs text-muted-foreground">
                {required ? (
                  <Trans>Clients without a valid certificate will be rejected</Trans>
                ) : (
                  <Trans>Certificate is optional — clients without one can still connect</Trans>
                )}
              </p>
            </div>
          </div>
        </div>
      )}

      <div className="flex items-center gap-3 pt-2">
        <Button onClick={handleSave} disabled={saving}>
          {saving ? t`Saving...` : t`Save`}
        </Button>
        {status === "saved" && (
          <Badge variant="outline" className="text-green-600 border-green-600">
            <Trans>Saved</Trans>
          </Badge>
        )}
        {status === "error" && (
          <Badge variant="outline" className="text-red-600 border-red-600">
            <Trans>Failed — check proxy status</Trans>
          </Badge>
        )}
      </div>
    </div>
  );
}

function ProxyAuthSection() {
  const { t } = useLingui();
  const isProxyConnected = useProxyStore((s) => s.isConnected);
  const [proxyAuthEnabled, setProxyAuthEnabled] = useState(false);
  const [proxyAuthUsername, setProxyAuthUsername] = useState("");
  const [proxyAuthPassword, setProxyAuthPassword] = useState("");
  const [proxyAuthSaving, setProxyAuthSaving] = useState(false);
  const [proxyAuthStatus, setProxyAuthStatus] = useState<"idle" | "saved" | "error">("idle");

  // store에서 설정 불러오기
  useEffect(() => {
    const config = useAppSettingsStore.getState().proxyAuthConfig;
    setProxyAuthEnabled(config.enabled);
    setProxyAuthUsername(config.username);
    setProxyAuthPassword(config.password);
  }, []);

  // 프록시 연결 시 설정 동기화
  useEffect(() => {
    if (isProxyConnected && proxyAuthEnabled) {
      updateProxyAuth({
        enabled: proxyAuthEnabled,
        username: proxyAuthUsername,
        password: proxyAuthPassword,
      }).catch(() => {});
    }
  }, [isProxyConnected]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleProxyAuthSave = useCallback(async () => {
    setProxyAuthSaving(true);
    setProxyAuthStatus("idle");

    try {
      const config: ProxyAuthConfig = {
        enabled: proxyAuthEnabled,
        username: proxyAuthUsername,
        password: proxyAuthPassword,
      };

      await updateProxyAuth(config);

      useAppSettingsStore.getState().setProxyAuthConfig(config);

      setProxyAuthStatus("saved");
      setTimeout(() => setProxyAuthStatus("idle"), 2000);
    } catch (e) {
      console.error("Proxy auth 설정 저장 실패:", e);
      setProxyAuthStatus("error");
    } finally {
      setProxyAuthSaving(false);
    }
  }, [proxyAuthEnabled, proxyAuthUsername, proxyAuthPassword]);

  return (
    <div className="border rounded-lg p-5 space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Proxy Authentication</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>Require authentication to use this proxy server</Trans>
          </p>
        </div>
        <Switch checked={proxyAuthEnabled} onCheckedChange={setProxyAuthEnabled} />
      </div>

      {proxyAuthEnabled && (
        <div className="space-y-4 pt-2">
          <div className="flex gap-3">
            <div className="flex-1">
              <label className="text-sm font-medium mb-1.5 block">
                <Trans>Username</Trans>
              </label>
              <Input
                placeholder={t`Username`}
                value={proxyAuthUsername}
                onChange={(e) => setProxyAuthUsername(e.target.value)}
              />
            </div>
            <div className="flex-1">
              <label className="text-sm font-medium mb-1.5 block">
                <Trans>Password</Trans>
              </label>
              <Input
                type="password"
                placeholder={t`Password`}
                value={proxyAuthPassword}
                onChange={(e) => setProxyAuthPassword(e.target.value)}
              />
            </div>
          </div>
          <p className="text-xs text-muted-foreground">
            <Trans>
              Clients must provide these credentials via Proxy-Authorization header (HTTP Basic) to
              use this proxy
            </Trans>
          </p>
        </div>
      )}

      {/* Save Button */}
      <div className="flex items-center gap-3 pt-2">
        <Button onClick={handleProxyAuthSave} disabled={proxyAuthSaving}>
          {proxyAuthSaving ? t`Saving...` : t`Save`}
        </Button>
        {proxyAuthStatus === "saved" && (
          <Badge variant="outline" className="text-green-600 border-green-600">
            <Trans>Saved</Trans>
          </Badge>
        )}
        {proxyAuthStatus === "error" && (
          <Badge variant="outline" className="text-red-600 border-red-600">
            <Trans>Failed — is the proxy running?</Trans>
          </Badge>
        )}
      </div>
    </div>
  );
}
