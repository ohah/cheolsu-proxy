import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { loadCatalog, locales, type Locale } from "@/shared/lib/i18n";
import { installCli, uninstallCli, checkCliInstalled } from "@/shared/api/proxy";
import {
  Button,
  Input,
  Switch,
  Badge,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue,
} from "@/shared/ui";

interface UpstreamProxyConfig {
  host: string;
  port: number;
  auth: { username: string; password: string } | null;
  bypass: string[];
}

export function SettingsPage() {
  const { t } = useLingui();
  const [enabled, setEnabled] = useState(false);
  const [host, setHost] = useState("");
  const [port, setPort] = useState("8080");
  const [useAuth, setUseAuth] = useState(false);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [bypass, setBypass] = useState("localhost, 127.0.0.1");
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<"idle" | "saved" | "error">("idle");
  const [locale, setLocale] = useState<Locale>(
    () => (localStorage.getItem("locale") as Locale) || "en",
  );
  const [cliInstalled, setCliInstalled] = useState(false);
  const [cliInstalling, setCliInstalling] = useState(false);
  const [cliMessage, setCliMessage] = useState("");

  const handleLocaleChange = useCallback(async (newLocale: string) => {
    const loc = newLocale as Locale;
    setLocale(loc);
    localStorage.setItem("locale", loc);
    await loadCatalog(loc);
  }, []);

  // CLI 설치 상태 확인
  useEffect(() => {
    checkCliInstalled().then(setCliInstalled);
  }, []);

  const handleInstallCli = useCallback(async () => {
    setCliInstalling(true);
    setCliMessage("");
    try {
      const msg = await installCli();
      setCliMessage(msg);
      setCliInstalled(true);
    } catch (e) {
      setCliMessage(String(e));
    } finally {
      setCliInstalling(false);
    }
  }, []);

  const handleUninstallCli = useCallback(async () => {
    setCliInstalling(true);
    setCliMessage("");
    try {
      const msg = await uninstallCli();
      setCliMessage(msg);
      setCliInstalled(false);
    } catch (e) {
      setCliMessage(String(e));
    } finally {
      setCliInstalling(false);
    }
  }, []);

  // 로컬 스토리지에서 설정 불러오기
  useEffect(() => {
    const saved = localStorage.getItem("upstream_proxy_config");
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        setEnabled(parsed.enabled ?? false);
        setHost(parsed.host ?? "");
        setPort(String(parsed.port ?? "8080"));
        setUseAuth(!!parsed.auth);
        setUsername(parsed.auth?.username ?? "");
        setPassword(parsed.auth?.password ?? "");
        setBypass((parsed.bypass ?? []).join(", "));
      } catch {
        // 파싱 실패 시 무시
      }
    }
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setStatus("idle");

    try {
      const config: UpstreamProxyConfig | null = enabled
        ? {
            host,
            port: Number.parseInt(port, 10) || 8080,
            auth: useAuth ? { username, password } : null,
            bypass: bypass
              .split(",")
              .map((s) => s.trim())
              .filter(Boolean),
          }
        : null;

      await invoke("update_upstream_proxy", { config });

      // 로컬 스토리지에 저장
      localStorage.setItem("upstream_proxy_config", JSON.stringify({ enabled, ...config }));

      setStatus("saved");
      setTimeout(() => setStatus("idle"), 2000);
    } catch (e) {
      console.error("Upstream proxy 설정 저장 실패:", e);
      setStatus("error");
    } finally {
      setSaving(false);
    }
  }, [enabled, host, port, useAuth, username, password, bypass]);

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

        {/* Language Section */}
        <div className="border rounded-lg p-5 space-y-5">
          <div>
            <h2 className="text-lg font-semibold">
              <Trans>Language</Trans>
            </h2>
          </div>
          <Select value={locale} onValueChange={handleLocaleChange}>
            <SelectTrigger className="w-48">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {Object.entries(locales).map(([code, name]) => (
                <SelectItem key={code} value={code}>
                  {name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* CLI Install Section */}
        <div className="border rounded-lg p-5 space-y-4">
          <div>
            <h2 className="text-lg font-semibold">
              <Trans>Terminal Command</Trans>
            </h2>
            <p className="text-sm text-muted-foreground">
              <Trans>
                Install the <code className="text-xs bg-muted px-1 py-0.5 rounded">cheolsu</code>{" "}
                command to use the TUI from your terminal
              </Trans>
            </p>
          </div>
          <div className="flex items-center gap-3">
            <Button onClick={handleInstallCli} disabled={cliInstalling}>
              {cliInstalling ? t`Installing...` : cliInstalled ? t`Reinstall` : t`Install`}
            </Button>
            {cliInstalled && (
              <Button variant="outline" onClick={handleUninstallCli} disabled={cliInstalling}>
                {t`Uninstall`}
              </Button>
            )}
            {cliInstalled && (
              <Badge variant="outline" className="text-green-600 border-green-600">
                <Trans>Installed</Trans>
              </Badge>
            )}
          </div>
          {cliMessage && <p className="text-xs text-muted-foreground">{cliMessage}</p>}
        </div>

        {/* Upstream Proxy Section */}
        <div className="border rounded-lg p-5 space-y-5">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-lg font-semibold">
                <Trans>Upstream Proxy</Trans>
              </h2>
              <p className="text-sm text-muted-foreground">
                <Trans>Route traffic through an external proxy server</Trans>
              </p>
            </div>
            <Switch checked={enabled} onCheckedChange={setEnabled} />
          </div>

          {enabled && (
            <div className="space-y-4 pt-2">
              {/* Host & Port */}
              <div className="flex gap-3">
                <div className="flex-1">
                  <label className="text-sm font-medium mb-1.5 block">
                    <Trans>Host</Trans>
                  </label>
                  <Input
                    placeholder={t`proxy.company.com`}
                    value={host}
                    onChange={(e) => setHost(e.target.value)}
                  />
                </div>
                <div className="w-28">
                  <label className="text-sm font-medium mb-1.5 block">
                    <Trans>Port</Trans>
                  </label>
                  <Input
                    type="number"
                    placeholder="8080"
                    value={port}
                    onChange={(e) => setPort(e.target.value)}
                  />
                </div>
              </div>

              {/* Authentication */}
              <div className="space-y-3">
                <div className="flex items-center gap-2">
                  <Switch checked={useAuth} onCheckedChange={setUseAuth} />
                  <label className="text-sm font-medium">
                    <Trans>Authentication</Trans>
                  </label>
                </div>
                {useAuth && (
                  <div className="flex gap-3 pl-1">
                    <div className="flex-1">
                      <Input
                        placeholder={t`Username`}
                        value={username}
                        onChange={(e) => setUsername(e.target.value)}
                      />
                    </div>
                    <div className="flex-1">
                      <Input
                        type="password"
                        placeholder={t`Password`}
                        value={password}
                        onChange={(e) => setPassword(e.target.value)}
                      />
                    </div>
                  </div>
                )}
              </div>

              {/* Bypass */}
              <div>
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>Bypass List</Trans>
                </label>
                <Input
                  placeholder={t`localhost, 127.0.0.1, *.internal.com`}
                  value={bypass}
                  onChange={(e) => setBypass(e.target.value)}
                />
                <p className="text-xs text-muted-foreground mt-1">
                  <Trans>
                    Comma-separated list of hosts to connect directly (supports *.domain.com
                    wildcards)
                  </Trans>
                </p>
              </div>
            </div>
          )}

          {/* Save Button */}
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
                <Trans>Failed — is the proxy running?</Trans>
              </Badge>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
