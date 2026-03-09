import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useTheme } from "next-themes";
import { loadCatalog, locales, type Locale } from "@/shared/lib/i18n";
import {
  installCli,
  uninstallCli,
  checkCliInstalled,
  checkCaInstalled,
  installCaCert,
  uninstallCaCert,
  getCaCertPath,
  updateThrottle,
  type ThrottleConfig,
} from "@/shared/api/proxy";
import { register, unregister, isRegistered } from "@tauri-apps/plugin-global-shortcut";
import { useProxyStore } from "@/shared/stores";
import { startProxyV2, stopProxyV2 } from "@/shared/api/proxy";
import { trayStore } from "@/shared/stores/tray-sync-store";
import { toast } from "sonner";
import {
  getStoredShortcut,
  setStoredShortcut,
  getShortcutEnabled,
  setShortcutEnabled,
} from "@/hooks/use-global-shortcut";
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

const THEME_OPTIONS = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
] as const;

const THROTTLE_PRESETS = [
  { value: "none", label: "None", config: null },
  {
    value: "gprs",
    label: "GPRS (50 KB/s)",
    config: { enabled: true, download_rate: 50 * 1024, upload_rate: 20 * 1024, latency_ms: 500 },
  },
  {
    value: "slow3g",
    label: "Slow 3G (500 KB/s)",
    config: {
      enabled: true,
      download_rate: 500 * 1024,
      upload_rate: 500 * 1024,
      latency_ms: 400,
    },
  },
  {
    value: "fast3g",
    label: "Fast 3G (1.6 MB/s)",
    config: {
      enabled: true,
      download_rate: 1_600 * 1024,
      upload_rate: 768 * 1024,
      latency_ms: 150,
    },
  },
  {
    value: "lte",
    label: "4G/LTE (4 MB/s)",
    config: {
      enabled: true,
      download_rate: 4 * 1024 * 1024,
      upload_rate: 3 * 1024 * 1024,
      latency_ms: 50,
    },
  },
  {
    value: "wifi",
    label: "WiFi (30 MB/s)",
    config: {
      enabled: true,
      download_rate: 30 * 1024 * 1024,
      upload_rate: 15 * 1024 * 1024,
      latency_ms: 2,
    },
  },
  { value: "custom", label: "Custom", config: null },
] as const;

export function SettingsPage() {
  const { t } = useLingui();
  const { theme, setTheme } = useTheme();
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
  const [caInstalled, setCaInstalled] = useState(false);
  const [caInstalling, setCaInstalling] = useState(false);
  const [caMessage, setCaMessage] = useState("");
  const [caCertPath, setCaCertPath] = useState("");

  // Global Shortcut state
  const [hotkeyEnabled, setHotkeyEnabled] = useState(() => getShortcutEnabled());
  const [hotkey, setHotkey] = useState(() => getStoredShortcut());
  const [isRecording, setIsRecording] = useState(false);
  const [hotkeyStatus, setHotkeyStatus] = useState<"idle" | "saved" | "error">("idle");

  const handleHotkeyRecord = useCallback(
    (e: React.KeyboardEvent) => {
      if (!isRecording) return;
      e.preventDefault();
      e.stopPropagation();

      const parts: string[] = [];
      if (e.metaKey || e.ctrlKey) parts.push("CommandOrControl");
      if (e.altKey) parts.push("Alt");
      if (e.shiftKey) parts.push("Shift");

      const key = e.key;
      // modifier 키만 누른 경우 무시
      if (["Control", "Meta", "Alt", "Shift"].includes(key)) return;

      // 알파벳/숫자/F키 등
      let keyName = key;
      if (key.length === 1) {
        keyName = key.toUpperCase();
      } else if (key === " ") {
        keyName = "Space";
      }

      parts.push(keyName);
      if (parts.length >= 2) {
        setHotkey(parts.join("+"));
        setIsRecording(false);
      }
    },
    [isRecording],
  );

  const handleHotkeySave = useCallback(async () => {
    try {
      // 기존 단축키 해제
      const oldShortcut = getStoredShortcut();
      try {
        const wasRegistered = await isRegistered(oldShortcut);
        if (wasRegistered) await unregister(oldShortcut);
      } catch {
        // 무시
      }

      setStoredShortcut(hotkey);
      setShortcutEnabled(hotkeyEnabled);

      if (hotkeyEnabled) {
        await register(hotkey, async (event) => {
          if (event.state === "Pressed") {
            const { isConnected, port } = useProxyStore.getState();
            try {
              if (isConnected) {
                await stopProxyV2();
                useProxyStore.getState().setConnected(false);
                await trayStore.set("proxyConnected", false);
                await trayStore.save();
                toast.info("Proxy stopped");
              } else {
                await startProxyV2(port);
                useProxyStore.getState().setConnected(true);
                await trayStore.set("proxyConnected", true);
                await trayStore.save();
                toast.success("Proxy started");
              }
            } catch {
              toast.error("Proxy toggle failed");
            }
          }
        });
      }

      setHotkeyStatus("saved");
      setTimeout(() => setHotkeyStatus("idle"), 2000);
    } catch (e) {
      console.error("Failed to save hotkey:", e);
      setHotkeyStatus("error");
    }
  }, [hotkey, hotkeyEnabled]);

  // Throttle state
  const [throttleEnabled, setThrottleEnabled] = useState(false);
  const [throttlePreset, setThrottlePreset] = useState("none");
  const [throttleDownload, setThrottleDownload] = useState(""); // KB/s
  const [throttleUpload, setThrottleUpload] = useState(""); // KB/s
  const [throttleLatency, setThrottleLatency] = useState("0"); // ms
  const [throttleSaving, setThrottleSaving] = useState(false);
  const [throttleStatus, setThrottleStatus] = useState<"idle" | "saved" | "error">("idle");

  const handleLocaleChange = useCallback(async (newLocale: string | null) => {
    if (!newLocale) return;
    const loc = newLocale as Locale;
    setLocale(loc);
    localStorage.setItem("locale", loc);
    await loadCatalog(loc);
  }, []);

  // CLI 설치 상태 확인
  useEffect(() => {
    checkCliInstalled().then(setCliInstalled);
  }, []);

  // CA 인증서 상태 확인
  useEffect(() => {
    checkCaInstalled()
      .then(setCaInstalled)
      .catch(() => setCaInstalled(false));
    getCaCertPath()
      .then(setCaCertPath)
      .catch(() => setCaCertPath(""));
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

  const handleInstallCa = useCallback(async () => {
    setCaInstalling(true);
    setCaMessage("");
    try {
      const msg = await installCaCert();
      setCaMessage(msg);
      setCaInstalled(true);
    } catch (e) {
      setCaMessage(String(e));
    } finally {
      setCaInstalling(false);
    }
  }, []);

  const handleUninstallCa = useCallback(async () => {
    setCaInstalling(true);
    setCaMessage("");
    try {
      const msg = await uninstallCaCert();
      setCaMessage(msg);
      setCaInstalled(false);
    } catch (e) {
      setCaMessage(String(e));
    } finally {
      setCaInstalling(false);
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

  // 스로틀링 설정 불러오기
  useEffect(() => {
    const saved = localStorage.getItem("throttle_config");
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        setThrottleEnabled(parsed.enabled ?? false);
        setThrottlePreset(parsed.preset ?? "none");
        setThrottleDownload(parsed.download ?? "");
        setThrottleUpload(parsed.upload ?? "");
        setThrottleLatency(String(parsed.latency ?? "0"));
      } catch {
        // 파싱 실패 시 무시
      }
    }
  }, []);

  const handleThrottleSave = useCallback(async () => {
    setThrottleSaving(true);
    setThrottleStatus("idle");

    try {
      let config: ThrottleConfig | null = null;

      if (throttleEnabled) {
        if (throttlePreset === "custom") {
          const dlRate = Number.parseInt(throttleDownload, 10);
          const ulRate = Number.parseInt(throttleUpload, 10);
          config = {
            enabled: true,
            download_rate: dlRate > 0 ? dlRate * 1024 : null, // KB/s → bytes/s
            upload_rate: ulRate > 0 ? ulRate * 1024 : null,
            latency_ms: Number.parseInt(throttleLatency, 10) || 0,
          };
        } else {
          const preset = THROTTLE_PRESETS.find((p) => p.value === throttlePreset);
          if (preset?.config) {
            config = preset.config;
          }
        }
      }

      await updateThrottle(config);

      localStorage.setItem(
        "throttle_config",
        JSON.stringify({
          enabled: throttleEnabled,
          preset: throttlePreset,
          download: throttleDownload,
          upload: throttleUpload,
          latency: throttleLatency,
        }),
      );

      setThrottleStatus("saved");
      setTimeout(() => setThrottleStatus("idle"), 2000);
    } catch (e) {
      console.error("스로틀링 설정 저장 실패:", e);
      setThrottleStatus("error");
    } finally {
      setThrottleSaving(false);
    }
  }, [throttleEnabled, throttlePreset, throttleDownload, throttleUpload, throttleLatency]);

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

        {/* Theme Section */}
        <div className="border rounded-lg p-5 space-y-5">
          <div>
            <h2 className="text-lg font-semibold">
              <Trans>Theme</Trans>
            </h2>
          </div>
          <Select
            value={theme}
            onValueChange={(v) => {
              if (v) setTheme(v);
            }}
          >
            <SelectTrigger className="w-48">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {THEME_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {opt.value === "system" ? t`System` : opt.value === "light" ? t`Light` : t`Dark`}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* CA Certificate Section */}
        <div className="border rounded-lg p-5 space-y-4">
          <div>
            <h2 className="text-lg font-semibold">
              <Trans>CA Certificate</Trans>
            </h2>
            <p className="text-sm text-muted-foreground">
              <Trans>
                Install the CA certificate to trust HTTPS traffic intercepted by the proxy
              </Trans>
            </p>
          </div>
          {caCertPath && (
            <p className="text-xs text-muted-foreground font-mono break-all">{caCertPath}</p>
          )}
          <div className="flex items-center gap-3">
            <Button onClick={handleInstallCa} disabled={caInstalling}>
              {caInstalling ? t`Installing...` : caInstalled ? t`Reinstall` : t`Install`}
            </Button>
            {caInstalled && (
              <Button variant="outline" onClick={handleUninstallCa} disabled={caInstalling}>
                {t`Uninstall`}
              </Button>
            )}
            {caInstalled && (
              <Badge variant="outline" className="text-green-600 border-green-600">
                <Trans>Trusted</Trans>
              </Badge>
            )}
            {!caInstalled && caCertPath && (
              <Badge variant="outline" className="text-yellow-600 border-yellow-600">
                <Trans>Not Trusted</Trans>
              </Badge>
            )}
            {!caCertPath && (
              <Badge variant="outline" className="text-muted-foreground">
                <Trans>Start proxy first</Trans>
              </Badge>
            )}
          </div>
          {caMessage && <p className="text-xs text-muted-foreground">{caMessage}</p>}
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

        {/* Global Shortcut Section */}
        <div className="border rounded-lg p-5 space-y-5">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-lg font-semibold">
                <Trans>Global Shortcut</Trans>
              </h2>
              <p className="text-sm text-muted-foreground">
                <Trans>Toggle proxy on/off with a global keyboard shortcut</Trans>
              </p>
            </div>
            <Switch
              checked={hotkeyEnabled}
              onCheckedChange={(checked) => {
                setHotkeyEnabled(checked);
              }}
            />
          </div>

          {hotkeyEnabled && (
            <div className="space-y-3 pt-2">
              <div>
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>Shortcut Key</Trans>
                </label>
                <div className="flex gap-3 items-center">
                  <div
                    tabIndex={0}
                    role="button"
                    className={`flex-1 h-9 px-3 border rounded-md flex items-center text-sm cursor-pointer focus:outline-none ${
                      isRecording
                        ? "border-primary ring-2 ring-primary/30 text-muted-foreground"
                        : "bg-background"
                    }`}
                    onKeyDown={handleHotkeyRecord}
                    onClick={() => setIsRecording(true)}
                    onBlur={() => setIsRecording(false)}
                  >
                    {isRecording ? (
                      <span className="text-muted-foreground">
                        <Trans>Press a key combination...</Trans>
                      </span>
                    ) : (
                      <ShortcutDisplay shortcut={hotkey} />
                    )}
                  </div>
                  <Button variant="outline" size="sm" onClick={() => setIsRecording(!isRecording)}>
                    {isRecording ? t`Cancel` : t`Change`}
                  </Button>
                </div>
              </div>
            </div>
          )}

          <div className="flex items-center gap-3 pt-2">
            <Button onClick={handleHotkeySave}>{t`Save`}</Button>
            {hotkeyStatus === "saved" && (
              <Badge variant="outline" className="text-green-600 border-green-600">
                <Trans>Saved</Trans>
              </Badge>
            )}
            {hotkeyStatus === "error" && (
              <Badge variant="outline" className="text-red-600 border-red-600">
                <Trans>Failed to register shortcut</Trans>
              </Badge>
            )}
          </div>
        </div>

        {/* Network Throttling Section */}
        <div className="border rounded-lg p-5 space-y-5">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-lg font-semibold">
                <Trans>Network Throttling</Trans>
              </h2>
              <p className="text-sm text-muted-foreground">
                <Trans>Simulate slow network conditions for testing</Trans>
              </p>
            </div>
            <Switch checked={throttleEnabled} onCheckedChange={setThrottleEnabled} />
          </div>

          {throttleEnabled && (
            <div className="space-y-4 pt-2">
              <div>
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>Profile</Trans>
                </label>
                <Select
                  value={throttlePreset}
                  onValueChange={(v) => {
                    if (v) setThrottlePreset(v);
                  }}
                >
                  <SelectTrigger className="w-64">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {THROTTLE_PRESETS.map((preset) => (
                      <SelectItem key={preset.value} value={preset.value}>
                        {preset.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              {throttlePreset === "custom" && (
                <div className="space-y-3">
                  <div className="flex gap-3">
                    <div className="flex-1">
                      <label className="text-sm font-medium mb-1.5 block">
                        <Trans>Download (KB/s)</Trans>
                      </label>
                      <Input
                        type="number"
                        placeholder={t`Unlimited`}
                        value={throttleDownload}
                        onChange={(e) => setThrottleDownload(e.target.value)}
                      />
                    </div>
                    <div className="flex-1">
                      <label className="text-sm font-medium mb-1.5 block">
                        <Trans>Upload (KB/s)</Trans>
                      </label>
                      <Input
                        type="number"
                        placeholder={t`Unlimited`}
                        value={throttleUpload}
                        onChange={(e) => setThrottleUpload(e.target.value)}
                      />
                    </div>
                    <div className="w-28">
                      <label className="text-sm font-medium mb-1.5 block">
                        <Trans>Latency (ms)</Trans>
                      </label>
                      <Input
                        type="number"
                        placeholder="0"
                        value={throttleLatency}
                        onChange={(e) => setThrottleLatency(e.target.value)}
                      />
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}

          <div className="flex items-center gap-3 pt-2">
            <Button onClick={handleThrottleSave} disabled={throttleSaving}>
              {throttleSaving ? t`Saving...` : t`Save`}
            </Button>
            {throttleStatus === "saved" && (
              <Badge variant="outline" className="text-green-600 border-green-600">
                <Trans>Saved</Trans>
              </Badge>
            )}
            {throttleStatus === "error" && (
              <Badge variant="outline" className="text-red-600 border-red-600">
                <Trans>Failed — is the proxy running?</Trans>
              </Badge>
            )}
          </div>
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

function ShortcutDisplay({ shortcut }: { shortcut: string }) {
  const parts = shortcut.split("+").map((part) => {
    switch (part) {
      case "CommandOrControl":
        return navigator.platform.includes("Mac") ? "\u2318" : "Ctrl";
      case "Shift":
        return "\u21E7";
      case "Alt":
        return navigator.platform.includes("Mac") ? "\u2325" : "Alt";
      case "Space":
        return "Space";
      default:
        return part;
    }
  });

  return (
    <div className="flex items-center gap-1">
      {parts.map((part, i) => (
        <span key={i}>
          {i > 0 && <span className="text-muted-foreground mx-0.5">+</span>}
          <kbd className="px-1.5 py-0.5 bg-muted border rounded text-xs font-mono">{part}</kbd>
        </span>
      ))}
    </div>
  );
}
