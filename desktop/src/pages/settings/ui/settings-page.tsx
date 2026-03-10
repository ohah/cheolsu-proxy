import { useState, useEffect, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { useProxyStore } from "@/shared/stores/proxy-store";
import { useSslProxyingStore } from "@/shared/stores/ssl-proxying-store";
import {
  updateProxyAuth,
  updateClientCertificate,
  type ProxyAuthConfig,
  type ClientCertConfig,
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

  const handleSelectCert = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Certificate", extensions: ["pem", "crt", "cer"] }],
    });
    if (selected) {
      setCertPath(selected as string);
      setCertStatus("idle");
    }
  }, []);

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

  const handleCertSave = useCallback(async () => {
    setCertSaving(true);
    setCertStatus("idle");
    try {
      if (certEnabled && certPath && keyPath) {
        await updateClientCertificate({
          cert_path: certPath,
          key_path: keyPath,
          enabled: true,
        });
      } else {
        await updateClientCertificate(
          certEnabled ? { cert_path: certPath, key_path: keyPath, enabled: false } : null,
        );
      }
      setCertStatus("saved");
    } catch {
      setCertStatus("error");
    } finally {
      setCertSaving(false);
    }
  }, [certEnabled, certPath, keyPath]);

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

          <p className="text-xs text-muted-foreground">
            <Trans>Supports PEM-encoded certificates and keys (RSA, ECDSA, PKCS#8)</Trans>
          </p>
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

function ProxyAuthSection() {
  const { t } = useLingui();
  const isProxyConnected = useProxyStore((s) => s.isConnected);
  const [proxyAuthEnabled, setProxyAuthEnabled] = useState(false);
  const [proxyAuthUsername, setProxyAuthUsername] = useState("");
  const [proxyAuthPassword, setProxyAuthPassword] = useState("");
  const [proxyAuthSaving, setProxyAuthSaving] = useState(false);
  const [proxyAuthStatus, setProxyAuthStatus] = useState<"idle" | "saved" | "error">("idle");

  // 로컬 스토리지에서 설정 불러오기
  useEffect(() => {
    const saved = localStorage.getItem("proxy_auth_config");
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        setProxyAuthEnabled(parsed.enabled ?? false);
        setProxyAuthUsername(parsed.username ?? "");
        setProxyAuthPassword(parsed.password ?? "");
      } catch {
        // 파싱 실패 시 무시
      }
    }
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

      localStorage.setItem("proxy_auth_config", JSON.stringify(config));

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
