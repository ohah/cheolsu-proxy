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
  updateQuickSettings,
  updateProxyAuth,
  updateClientCertificate,
  getCertDownloadInfo,
  type ThrottleConfig,
  type CertDownloadInfo,
  type ProxyAuthConfig,
  type ClientCertConfig,
} from "@/shared/api/proxy";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { useProxyStore } from "@/shared/stores/proxy-store";
import { useSslProxyingStore } from "@/shared/stores/ssl-proxying-store";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import {
  getStoredShortcut,
  setStoredShortcut,
  getShortcutEnabled,
  setShortcutEnabled,
  registerShortcut,
  unregisterShortcut,
} from "@/shared/lib/global-shortcut";
import { toggleProxy } from "@/features/proxy-toggle";
import { platform } from "@tauri-apps/plugin-os";
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

const CERT_DOWNLOAD_URL = "http://cheolsu.proxy/ssl";
const CERT_DOWNLOAD_PATHS = {
  pem: "/ssl/pem",
  der: "/ssl/der",
  universal: "/ssl/ca.crt",
} as const;

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
  const [locale, setLocale] = useState<Locale>(() => useAppSettingsStore.getState().locale || "en");
  const [cliInstalled, setCliInstalled] = useState(false);
  const [cliInstalling, setCliInstalling] = useState(false);
  const [cliMessage, setCliMessage] = useState("");
  const [caInstalled, setCaInstalled] = useState(false);
  const [caInstalling, setCaInstalling] = useState(false);
  const [caMessage, setCaMessage] = useState("");
  const [caCertPath, setCaCertPath] = useState("");
  const [certDownloadInfo, setCertDownloadInfo] = useState<CertDownloadInfo | null>(null);
  const [certDownloadLoading, setCertDownloadLoading] = useState(false);
  const [certUrlCopied, setCertUrlCopied] = useState(false);
  const [showIosGuide, setShowIosGuide] = useState(false);
  const [showAndroidGuide, setShowAndroidGuide] = useState(false);
  const proxyPort = useProxyStore((s) => s.port);
  const isProxyConnected = useProxyStore((s) => s.isConnected);

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
      const hasCtrlOrCmd = e.metaKey || e.ctrlKey;
      const hasAlt = e.altKey;
      if (hasCtrlOrCmd) parts.push("CommandOrControl");
      if (hasAlt) parts.push("Alt");
      if (e.shiftKey) parts.push("Shift");

      const key = e.key;
      // modifier 키만 누른 경우 무시
      if (["Control", "Meta", "Alt", "Shift"].includes(key)) return;

      // CommandOrControl 또는 Alt 필수 (Shift 단독은 일반 타이핑과 충돌)
      if (!hasCtrlOrCmd && !hasAlt) return;

      // 알파벳/숫자/F키 등
      let keyName = key;
      if (key.length === 1) {
        keyName = key.toUpperCase();
      } else if (key === " ") {
        keyName = "Space";
      }

      parts.push(keyName);
      setHotkey(parts.join("+"));
      setIsRecording(false);
    },
    [isRecording],
  );

  const handleHotkeySave = useCallback(async () => {
    try {
      setStoredShortcut(hotkey);
      setShortcutEnabled(hotkeyEnabled);

      if (hotkeyEnabled) {
        // 훅의 registerShortcut이 기존 단축키 해제 + 새 단축키 등록을 모두 처리
        await registerShortcut(hotkey, toggleProxy);
      } else {
        await unregisterShortcut();
      }

      setHotkeyStatus("saved");
      setTimeout(() => setHotkeyStatus("idle"), 2000);
    } catch {
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

  // Quick Settings state
  const [noCaching, setNoCaching] = useState(
    () => useAppSettingsStore.getState().quickSettingsNoCaching,
  );
  const [blockCookies, setBlockCookies] = useState(
    () => useAppSettingsStore.getState().quickSettingsBlockCookies,
  );
  const [noGzip, setNoGzip] = useState(() => useAppSettingsStore.getState().quickSettingsNoGzip);

  const [autosaveEnabled, setAutosaveEnabled] = useState(
    () => useAppSettingsStore.getState().autosaveSession,
  );

  const handleAutosaveChange = useCallback((checked: boolean) => {
    setAutosaveEnabled(checked);
    useAppSettingsStore.getState().setAutosaveSession(checked);
  }, []);

  const handleLocaleChange = useCallback(async (newLocale: string | null) => {
    if (!newLocale) return;
    const loc = newLocale as Locale;
    setLocale(loc);
    useAppSettingsStore.getState().setLocale(loc);
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

  // 프록시 실행 중일 때 인증서 다운로드 정보 로드
  const loadCertDownloadInfo = useCallback(async () => {
    setCertDownloadLoading(true);
    try {
      const info = await getCertDownloadInfo(proxyPort);
      setCertDownloadInfo(info);
    } catch (e) {
      console.error("인증서 다운로드 정보 로드 실패:", e);
      setCertDownloadInfo(null);
    } finally {
      setCertDownloadLoading(false);
    }
  }, [proxyPort]);

  useEffect(() => {
    if (isProxyConnected) {
      loadCertDownloadInfo();
    } else {
      setCertDownloadInfo(null);
    }
  }, [isProxyConnected, loadCertDownloadInfo]);

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
    const saved = useAppSettingsStore.getState().throttleConfig;
    setThrottleEnabled(saved.enabled);
    setThrottlePreset(saved.preset);
    setThrottleDownload(saved.download);
    setThrottleUpload(saved.upload);
    setThrottleLatency(saved.latency);
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

      useAppSettingsStore.getState().setThrottleConfig({
        enabled: throttleEnabled,
        preset: throttlePreset,
        download: throttleDownload,
        upload: throttleUpload,
        latency: throttleLatency,
      });

      setThrottleStatus("saved");
      setTimeout(() => setThrottleStatus("idle"), 2000);
    } catch (e) {
      console.error("스로틀링 설정 저장 실패:", e);
      setThrottleStatus("error");
    } finally {
      setThrottleSaving(false);
    }
  }, [throttleEnabled, throttlePreset, throttleDownload, throttleUpload, throttleLatency]);

  // Quick Settings 변경 시 즉시 적용
  const handleNoCachingChange = useCallback(async (checked: boolean) => {
    setNoCaching(checked);
    useAppSettingsStore.getState().setQuickSettingsNoCaching(checked);
    setBlockCookies((currentBlockCookies) => {
      setNoGzip((currentNoGzip) => {
        updateQuickSettings(checked, currentBlockCookies, currentNoGzip).catch((e) => {
          console.error("No Caching 설정 실패:", e);
        });
        return currentNoGzip;
      });
      return currentBlockCookies;
    });
  }, []);

  const handleBlockCookiesChange = useCallback(async (checked: boolean) => {
    setBlockCookies(checked);
    useAppSettingsStore.getState().setQuickSettingsBlockCookies(checked);
    setNoCaching((currentNoCaching) => {
      setNoGzip((currentNoGzip) => {
        updateQuickSettings(currentNoCaching, checked, currentNoGzip).catch((e) => {
          console.error("Block Cookies 설정 실패:", e);
        });
        return currentNoGzip;
      });
      return currentNoCaching;
    });
  }, []);

  const handleNoGzipChange = useCallback(async (checked: boolean) => {
    setNoGzip(checked);
    useAppSettingsStore.getState().setQuickSettingsNoGzip(checked);
    setNoCaching((currentNoCaching) => {
      setBlockCookies((currentBlockCookies) => {
        updateQuickSettings(currentNoCaching, currentBlockCookies, checked).catch((e) => {
          console.error("No Gzip 설정 실패:", e);
        });
        return currentBlockCookies;
      });
      return currentNoCaching;
    });
  }, []);

  // 프록시 연결 시 Quick Settings 동기화
  useEffect(() => {
    if (isProxyConnected) {
      updateQuickSettings(noCaching, blockCookies, noGzip).catch(() => {});
    }
  }, [isProxyConnected]); // eslint-disable-line react-hooks/exhaustive-deps

  // store에서 설정 불러오기
  useEffect(() => {
    const saved = useAppSettingsStore.getState().upstreamProxyConfig;
    setEnabled(saved.enabled);
    setHost(saved.host);
    setPort(String(saved.port));
    setUseAuth(!!saved.auth);
    setUsername(saved.auth?.username ?? "");
    setPassword(saved.auth?.password ?? "");
    setBypass(saved.bypass.join(", "));
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

      // store에 저장
      useAppSettingsStore.getState().setUpstreamProxyConfig({
        enabled,
        host: config?.host ?? "",
        port: config?.port ?? 8080,
        auth: config?.auth ?? null,
        bypass: config?.bypass ?? [],
      });

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

        {/* Remote Device Certificate Section */}
        <div className="border rounded-lg p-5 space-y-4">
          <div>
            <h2 className="text-lg font-semibold">
              <Trans>Remote Device Certificate</Trans>
            </h2>
            <p className="text-sm text-muted-foreground">
              <Trans>
                Install the CA certificate on external devices (mobile, tablet) to intercept HTTPS
                traffic
              </Trans>
            </p>
          </div>

          {!isProxyConnected ? (
            <div className="text-sm text-muted-foreground">
              <Badge variant="outline" className="text-yellow-600 border-yellow-600">
                <Trans>Start proxy first</Trans>
              </Badge>
            </div>
          ) : certDownloadLoading ? (
            <div className="text-sm text-muted-foreground">
              <Trans>Loading...</Trans>
            </div>
          ) : certDownloadInfo ? (
            <div className="space-y-4">
              {/* 설치 안내 */}
              <div className="bg-muted/50 rounded-lg p-4 text-sm space-y-2">
                <p className="font-medium">
                  <Trans>Setup Instructions:</Trans>
                </p>
                <ol className="list-decimal list-inside space-y-1 text-muted-foreground">
                  <li>
                    <Trans>
                      Set Wi-Fi proxy on your device to{" "}
                      <code className="bg-muted px-1 py-0.5 rounded text-xs font-mono">
                        {certDownloadInfo.local_ips[0] || "127.0.0.1"}:{certDownloadInfo.port}
                      </code>
                    </Trans>
                  </li>
                  <li>
                    <Trans>
                      Open{" "}
                      <code className="bg-muted px-1 py-0.5 rounded text-xs font-mono">
                        {CERT_DOWNLOAD_URL}
                      </code>{" "}
                      in your device browser
                    </Trans>
                  </li>
                  <li>
                    <Trans>Install and trust the downloaded certificate</Trans>
                  </li>
                </ol>
              </div>

              {/* 모바일 인증서 설치 카드 */}
              <div className="border rounded-lg p-4 space-y-3 bg-muted/30">
                <div className="flex items-center justify-between">
                  <h3 className="text-sm font-semibold">
                    <Trans>Mobile Certificate Install</Trans>
                  </h3>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={async () => {
                      try {
                        await navigator.clipboard.writeText(CERT_DOWNLOAD_URL);
                        setCertUrlCopied(true);
                        setTimeout(() => setCertUrlCopied(false), 2000);
                      } catch {
                        // Fallback for HTTP or unfocused contexts
                        const textarea = document.createElement("textarea");
                        textarea.value = CERT_DOWNLOAD_URL;
                        textarea.style.position = "fixed";
                        textarea.style.opacity = "0";
                        document.body.appendChild(textarea);
                        textarea.select();
                        try {
                          document.execCommand("copy");
                          setCertUrlCopied(true);
                          setTimeout(() => setCertUrlCopied(false), 2000);
                        } catch {
                          console.error("클립보드 복사 실패");
                        }
                        document.body.removeChild(textarea);
                      }
                    }}
                  >
                    {certUrlCopied ? t`Copied!` : t`Copy URL`}
                  </Button>
                </div>

                <div className="font-mono text-sm bg-muted px-3 py-2 rounded text-center select-all">
                  {CERT_DOWNLOAD_URL}
                </div>

                <p className="text-xs text-muted-foreground">
                  <Trans>
                    Open this URL in your mobile browser after setting the Wi-Fi proxy. The page
                    automatically detects your device and provides the correct certificate format.
                  </Trans>
                </p>

                <div className="text-xs text-muted-foreground space-y-1">
                  <p>
                    <code className="bg-muted px-1 py-0.5 rounded">{CERT_DOWNLOAD_PATHS.pem}</code>{" "}
                    — PEM {t`format`} (iOS)
                  </p>
                  <p>
                    <code className="bg-muted px-1 py-0.5 rounded">{CERT_DOWNLOAD_PATHS.der}</code>{" "}
                    — DER {t`format`} (Android)
                  </p>
                  <p>
                    <code className="bg-muted px-1 py-0.5 rounded">
                      {CERT_DOWNLOAD_PATHS.universal}
                    </code>{" "}
                    — {t`Universal format`}
                  </p>
                </div>

                {/* iOS 가이드 */}
                <div className="border rounded-md">
                  <button
                    type="button"
                    aria-expanded={showIosGuide}
                    aria-controls="ios-guide-content"
                    className="w-full text-left px-3 py-2 text-sm font-medium flex items-center justify-between hover:bg-muted/50 transition-colors"
                    onClick={() => setShowIosGuide(!showIosGuide)}
                  >
                    <span>iOS {t`Install Guide`}</span>
                    <span className="text-muted-foreground">{showIosGuide ? "▲" : "▼"}</span>
                  </button>
                  {showIosGuide && (
                    <div
                      id="ios-guide-content"
                      className="px-3 pb-3 text-xs text-muted-foreground space-y-1"
                    >
                      <ol className="list-decimal list-inside space-y-1">
                        <li>
                          <Trans>
                            Open <strong>{CERT_DOWNLOAD_URL}</strong> in Safari
                          </Trans>
                        </li>
                        <li>
                          <Trans>
                            Tap "Allow" when prompted to download the configuration profile
                          </Trans>
                        </li>
                        <li>
                          <Trans>
                            Go to <strong>Settings → General → VPN & Device Management</strong>
                          </Trans>
                        </li>
                        <li>
                          <Trans>Select the downloaded profile and tap "Install"</Trans>
                        </li>
                        <li>
                          <Trans>
                            Go to{" "}
                            <strong>Settings → General → About → Certificate Trust Settings</strong>
                          </Trans>
                        </li>
                        <li>
                          <Trans>Enable full trust for the Cheolsu Proxy CA certificate</Trans>
                        </li>
                      </ol>
                    </div>
                  )}
                </div>

                {/* Android 가이드 */}
                <div className="border rounded-md">
                  <button
                    type="button"
                    aria-expanded={showAndroidGuide}
                    aria-controls="android-guide-content"
                    className="w-full text-left px-3 py-2 text-sm font-medium flex items-center justify-between hover:bg-muted/50 transition-colors"
                    onClick={() => setShowAndroidGuide(!showAndroidGuide)}
                  >
                    <span>Android {t`Install Guide`}</span>
                    <span className="text-muted-foreground">{showAndroidGuide ? "▲" : "▼"}</span>
                  </button>
                  {showAndroidGuide && (
                    <div
                      id="android-guide-content"
                      className="px-3 pb-3 text-xs text-muted-foreground space-y-1"
                    >
                      <ol className="list-decimal list-inside space-y-1">
                        <li>
                          <Trans>
                            Open <strong>{CERT_DOWNLOAD_URL}</strong> in Chrome
                          </Trans>
                        </li>
                        <li>
                          <Trans>The DER certificate file will download automatically</Trans>
                        </li>
                        <li>
                          <Trans>
                            Go to <strong>Settings → Security → Encryption & credentials</strong>
                          </Trans>
                        </li>
                        <li>
                          <Trans>Tap "Install a certificate" → "CA certificate"</Trans>
                        </li>
                        <li>
                          <Trans>Select the downloaded certificate file and confirm</Trans>
                        </li>
                      </ol>
                    </div>
                  )}
                </div>
              </div>

              {/* QR 코드 + URL 정보 */}
              <div className="flex gap-6 items-start">
                {/* QR 코드 */}
                <div className="flex-shrink-0">
                  <div className="bg-white p-2 rounded-lg border">
                    <img
                      src={`data:image/png;base64,${certDownloadInfo.qr_code_base64}`}
                      alt={`Certificate download QR code for ${CERT_DOWNLOAD_URL}`}
                      className="w-32 h-32"
                      style={{ imageRendering: "pixelated" }}
                    />
                  </div>
                  <p className="text-xs text-muted-foreground mt-1 text-center">
                    <Trans>Scan for proxy info</Trans>
                  </p>
                </div>

                {/* URL 정보 */}
                <div className="flex-1 space-y-3">
                  <div>
                    <label className="text-xs font-medium text-muted-foreground block mb-1">
                      <Trans>Proxy Address</Trans>
                    </label>
                    {certDownloadInfo.local_ips.map((ip) => (
                      <div key={ip} className="font-mono text-sm bg-muted px-3 py-1.5 rounded mb-1">
                        {ip}:{certDownloadInfo.port}
                      </div>
                    ))}
                  </div>
                  <div>
                    <label className="text-xs font-medium text-muted-foreground block mb-1">
                      <Trans>Certificate Download URL</Trans>
                    </label>
                    <div className="font-mono text-sm bg-muted px-3 py-1.5 rounded">
                      {CERT_DOWNLOAD_URL}
                    </div>
                  </div>
                  <Button variant="outline" size="sm" onClick={loadCertDownloadInfo}>
                    {t`Refresh`}
                  </Button>
                </div>
              </div>
            </div>
          ) : null}
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

        {/* SSL Proxying Section */}
        <SslProxyingSection />

        {/* Quick Settings Section */}
        <div className="border rounded-lg p-5 space-y-5">
          <div>
            <h2 className="text-lg font-semibold">
              <Trans>Quick Settings</Trans>
            </h2>
            <p className="text-sm text-muted-foreground">
              <Trans>Quick toggles for common proxy behaviors</Trans>
            </p>
          </div>

          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm font-medium">
                  <Trans>No Caching</Trans>
                </label>
                <p className="text-xs text-muted-foreground">
                  <Trans>
                    Prevent caching by removing conditional headers and adding no-cache directives
                  </Trans>
                </p>
              </div>
              <Switch checked={noCaching} onCheckedChange={handleNoCachingChange} />
            </div>

            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm font-medium">
                  <Trans>Block Cookies</Trans>
                </label>
                <p className="text-xs text-muted-foreground">
                  <Trans>
                    Remove Cookie headers from requests and Set-Cookie headers from responses
                  </Trans>
                </p>
              </div>
              <Switch checked={blockCookies} onCheckedChange={handleBlockCookiesChange} />
            </div>

            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm font-medium">
                  <Trans>No Gzip</Trans>
                </label>
                <p className="text-xs text-muted-foreground">
                  <Trans>
                    Remove Accept-Encoding header from requests to prevent compressed responses
                  </Trans>
                </p>
              </div>
              <Switch checked={noGzip} onCheckedChange={handleNoGzipChange} />
            </div>

            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm font-medium">
                  <Trans>Auto Save Session</Trans>
                </label>
                <p className="text-xs text-muted-foreground">
                  <Trans>
                    Automatically save the current session when the app closes and restore it on
                    next launch
                  </Trans>
                </p>
              </div>
              <Switch checked={autosaveEnabled} onCheckedChange={handleAutosaveChange} />
            </div>
          </div>
        </div>

        {/* Proxy Authentication Section */}
        <ProxyAuthSection />

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

        {/* Client Certificate (mTLS) Section */}
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

  // store에서 설정 불러오기
  useEffect(() => {
    const saved = useAppSettingsStore.getState().proxyAuthConfig;
    setProxyAuthEnabled(saved.enabled);
    setProxyAuthUsername(saved.username);
    setProxyAuthPassword(saved.password);
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

const isMac = platform() === "macos";

function ShortcutDisplay({ shortcut }: { shortcut: string }) {
  const parts = shortcut.split("+").map((part) => {
    switch (part) {
      case "CommandOrControl":
        return isMac ? "\u2318" : "Ctrl";
      case "Shift":
        return "\u21E7";
      case "Alt":
        return isMac ? "\u2325" : "Alt";
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
