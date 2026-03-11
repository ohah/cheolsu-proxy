import { useState, useEffect, useCallback, useRef } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import { useSslProxyingStore } from "@/shared/stores/ssl-proxying-store";
import {
  updateProxyAuth,
  updateClientCertificate,
  updateRequestClientCert,
  updateThrottle,
  updateConnectionStrategy,
  updateQuickSettings,
  parseCertificateInfo,
  type ThrottleConfig,
  type ConnectionStrategy,
  type CertificateInfo,
} from "@/shared/api/proxy";
import {
  setStoredShortcut,
  setShortcutEnabled,
  registerShortcut,
  unregisterShortcut,
} from "@/shared/lib/global-shortcut";
import { toggleProxy } from "@/features/proxy-toggle";
import { Button, Input, Switch, Badge } from "@/shared/ui";
import { Select, SelectTrigger, SelectContent, SelectItem, SelectValue } from "@/shared/ui";
import { platform } from "@tauri-apps/plugin-os";
import { GeneralSettings, CertificateSettings, CliSettings } from "./components";
import { SettingsFormProvider, useSettingsForm, type SettingsFormValues } from "./settings-form";

const isMac = platform() === "macos";

// --- Throttle presets ---
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
    config: { enabled: true, download_rate: 500 * 1024, upload_rate: 500 * 1024, latency_ms: 400 },
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

const STRATEGY_OPTIONS: { value: ConnectionStrategy; label: string }[] = [
  { value: "lazy", label: "Lazy" },
  { value: "eager", label: "Eager" },
  { value: "eager_with_fallback", label: "Eager with Fallback" },
];

type SettingsCategory = "general" | "certificate" | "network" | "security" | "tools";
const CATEGORIES: SettingsCategory[] = ["general", "certificate", "network", "security", "tools"];

// =============================================================================
// Save logic — only saves dirty sections, collects errors per-section
// =============================================================================
interface SaveResult {
  section: string;
  success: boolean;
  error?: unknown;
}

async function saveAllSettings(
  data: SettingsFormValues,
  dirtyFields: Partial<Record<keyof SettingsFormValues, unknown>>,
) {
  const store = useAppSettingsStore.getState();
  const results: SaveResult[] = [];

  // Quick Settings
  if (dirtyFields.quickSettings) {
    try {
      await updateQuickSettings(
        data.quickSettings.noCaching,
        data.quickSettings.blockCookies,
        data.quickSettings.noGzip,
      );
      store.setQuickSettings({
        quickSettingsNoCaching: data.quickSettings.noCaching,
        quickSettingsBlockCookies: data.quickSettings.blockCookies,
        quickSettingsNoGzip: data.quickSettings.noGzip,
      });
      store.setAutosaveSession(data.quickSettings.autosaveSession);
      store.setShowConnectRequests(data.quickSettings.showConnectRequests);
      results.push({ section: "quickSettings", success: true });
    } catch (error) {
      results.push({ section: "quickSettings", success: false, error });
    }
  }

  // Throttle
  if (dirtyFields.throttle) {
    try {
      let throttleConfig: ThrottleConfig | null = null;
      if (data.throttle.enabled) {
        if (data.throttle.preset === "custom") {
          const dl = Number.parseInt(data.throttle.download, 10);
          const ul = Number.parseInt(data.throttle.upload, 10);
          throttleConfig = {
            enabled: true,
            download_rate: dl > 0 ? dl * 1024 : null,
            upload_rate: ul > 0 ? ul * 1024 : null,
            latency_ms: Number.parseInt(data.throttle.latency, 10) || 0,
          };
        } else {
          const preset = THROTTLE_PRESETS.find((p) => p.value === data.throttle.preset);
          if (preset?.config) throttleConfig = preset.config;
        }
      }
      await updateThrottle(throttleConfig);
      store.setThrottleConfig(data.throttle);
      results.push({ section: "throttle", success: true });
    } catch (error) {
      results.push({ section: "throttle", success: false, error });
    }
  }

  // Connection Strategy
  if (dirtyFields.connectionStrategy) {
    try {
      await updateConnectionStrategy(data.connectionStrategy);
      store.setConnectionStrategy(data.connectionStrategy);
      results.push({ section: "connectionStrategy", success: true });
    } catch (error) {
      results.push({ section: "connectionStrategy", success: false, error });
    }
  }

  // Upstream Proxy
  if (dirtyFields.upstreamProxy) {
    try {
      const upstreamConfig = data.upstreamProxy.enabled
        ? {
            host: data.upstreamProxy.host,
            port: Number.parseInt(data.upstreamProxy.port, 10) || 8080,
            auth: data.upstreamProxy.useAuth
              ? { username: data.upstreamProxy.username, password: data.upstreamProxy.password }
              : null,
            bypass: data.upstreamProxy.bypass
              .split(",")
              .map((s) => s.trim())
              .filter(Boolean),
          }
        : null;
      await invoke("update_upstream_proxy", { config: upstreamConfig });
      store.setUpstreamProxyConfig({
        enabled: data.upstreamProxy.enabled,
        ...(upstreamConfig ?? {
          host: data.upstreamProxy.host,
          port: Number.parseInt(data.upstreamProxy.port, 10) || 8080,
          auth: null,
          bypass: [],
        }),
      });
      results.push({ section: "upstreamProxy", success: true });
    } catch (error) {
      results.push({ section: "upstreamProxy", success: false, error });
    }
  }

  // Proxy Auth
  if (dirtyFields.proxyAuth) {
    try {
      const authConfig = {
        enabled: data.proxyAuth.enabled,
        username: data.proxyAuth.username,
        password: data.proxyAuth.password,
      };
      await updateProxyAuth(authConfig);
      store.setProxyAuthConfig(authConfig);
      results.push({ section: "proxyAuth", success: true });
    } catch (error) {
      results.push({ section: "proxyAuth", success: false, error });
    }
  }

  // Shortcut
  if (dirtyFields.shortcut) {
    try {
      setStoredShortcut(data.shortcut.key);
      setShortcutEnabled(data.shortcut.enabled);
      store.setProxyToggleShortcut(data.shortcut.key);
      store.setProxyToggleShortcutEnabled(data.shortcut.enabled);
      if (data.shortcut.enabled) {
        await registerShortcut(data.shortcut.key, toggleProxy);
      } else {
        await unregisterShortcut();
      }
      results.push({ section: "shortcut", success: true });
    } catch (error) {
      results.push({ section: "shortcut", success: false, error });
    }
  }

  // Client Certificate
  if (dirtyFields.clientCert) {
    try {
      if (data.clientCert.enabled && data.clientCert.certPath && data.clientCert.keyPath) {
        await updateClientCertificate({
          cert_path: data.clientCert.certPath,
          key_path: data.clientCert.keyPath,
          enabled: true,
          domain_certs: data.clientCert.domainCerts,
        });
      } else {
        await updateClientCertificate(
          data.clientCert.enabled
            ? {
                cert_path: data.clientCert.certPath,
                key_path: data.clientCert.keyPath,
                enabled: false,
                domain_certs: data.clientCert.domainCerts,
              }
            : null,
        );
      }
      results.push({ section: "clientCert", success: true });
    } catch (error) {
      results.push({ section: "clientCert", success: false, error });
    }
  }

  // Request Client Cert
  if (dirtyFields.requestClientCert) {
    try {
      if (data.requestClientCert.enabled) {
        await updateRequestClientCert({
          enabled: true,
          ca_cert_path: data.requestClientCert.caCertPath || null,
          required: data.requestClientCert.required,
        });
      } else {
        await updateRequestClientCert(null);
      }
      results.push({ section: "requestClientCert", success: true });
    } catch (error) {
      results.push({ section: "requestClientCert", success: false, error });
    }
  }

  // SSL Proxying
  if (dirtyFields.sslProxying) {
    try {
      const sslStore = useSslProxyingStore.getState();
      sslStore.setMode(data.sslProxying.mode);
      sslStore.setFromDaemon(data.sslProxying.mode, data.sslProxying.entries);
      await sslStore.syncToProxy();
      results.push({ section: "sslProxying", success: true });
    } catch (error) {
      results.push({ section: "sslProxying", success: false, error });
    }
  }

  const failures = results.filter((r) => !r.success);
  if (failures.length > 0) {
    console.error("Some sections failed to save:", failures);
    throw new Error(`Failed to save: ${failures.map((f) => f.section).join(", ")}`);
  }
}

// =============================================================================
// Page
// =============================================================================
export function SettingsPage() {
  return (
    <SettingsFormProvider>
      <SettingsPageInner />
    </SettingsFormProvider>
  );
}

function SettingsPageInner() {
  const { t } = useLingui();
  const [activeCategory, setActiveCategory] = useState<SettingsCategory>("general");
  const form = useSettingsForm();
  const { isDirty, isSubmitting, dirtyFields } = form.formState;
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved" | "error">("idle");
  const dirtyFieldsRef = useRef(dirtyFields);
  dirtyFieldsRef.current = dirtyFields;

  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const sectionRefs = useRef<Record<SettingsCategory, HTMLDivElement | null>>({
    general: null,
    certificate: null,
    network: null,
    security: null,
    tools: null,
  });

  // Track which section is visible via IntersectionObserver
  useEffect(() => {
    const container = scrollContainerRef.current;
    if (!container) return;

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            const cat = entry.target.getAttribute("data-category") as SettingsCategory | null;
            if (cat) setActiveCategory(cat);
          }
        }
      },
      { root: container, rootMargin: "-20% 0px -70% 0px", threshold: 0 },
    );

    for (const cat of CATEGORIES) {
      const el = sectionRefs.current[cat];
      if (el) observer.observe(el);
    }

    return () => observer.disconnect();
  }, []);

  const handleSave = useCallback(
    async (data: SettingsFormValues) => {
      setSaveStatus("idle");
      try {
        await saveAllSettings(data, dirtyFieldsRef.current);
        form.reset(data);
        setSaveStatus("saved");
        setTimeout(() => setSaveStatus("idle"), 2000);
      } catch (e) {
        console.error("Settings save failed:", e);
        setSaveStatus("error");
        setTimeout(() => setSaveStatus("idle"), 3000);
      }
    },
    [form],
  );

  const handleScrollTo = useCallback((cat: SettingsCategory) => {
    const el = sectionRefs.current[cat];
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  }, []);

  const categoryLabels: Record<SettingsCategory, string> = {
    general: t`General`,
    certificate: t`Certificate`,
    network: t`Network`,
    security: t`Security`,
    tools: t`Tools`,
  };

  return (
    <form onSubmit={form.handleSubmit(handleSave)} className="flex-1 flex h-full overflow-hidden">
      {/* Sidebar */}
      <nav className="w-48 flex-shrink-0 border-r p-4 space-y-1">
        {CATEGORIES.map((cat) => (
          <button
            key={cat}
            type="button"
            onClick={() => handleScrollTo(cat)}
            className={`w-full text-left px-3 py-2 rounded-md text-sm transition-colors ${
              activeCategory === cat
                ? "bg-accent text-accent-foreground font-medium"
                : "text-muted-foreground hover:bg-accent/50 hover:text-foreground"
            }`}
          >
            {categoryLabels[cat]}
          </button>
        ))}
      </nav>

      {/* Content */}
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        {/* Sticky header */}
        <div className="flex items-center justify-between px-6 py-4 border-b flex-shrink-0">
          <div>
            <h1 className="text-2xl font-bold text-foreground">
              <Trans>Settings</Trans>
            </h1>
            <p className="text-sm text-muted-foreground">
              <Trans>Proxy configuration and preferences</Trans>
            </p>
          </div>
          <div className="flex items-center gap-3">
            {saveStatus === "saved" && (
              <Badge variant="outline" className="text-green-600 border-green-600">
                <Trans>Saved</Trans>
              </Badge>
            )}
            {saveStatus === "error" && (
              <Badge variant="outline" className="text-red-600 border-red-600">
                <Trans>Save failed</Trans>
              </Badge>
            )}
            {isDirty && (
              <Button type="submit" disabled={isSubmitting} size="sm">
                {isSubmitting ? t`Saving...` : t`Save Changes`}
              </Button>
            )}
          </div>
        </div>

        {/* Scrollable content — all sections visible */}
        <div ref={scrollContainerRef} className="flex-1 overflow-auto p-6 space-y-10">
          <div
            ref={(el) => {
              sectionRefs.current.general = el;
            }}
            data-category="general"
            className="space-y-6"
          >
            <h2 className="text-xl font-semibold text-foreground">{categoryLabels.general}</h2>
            <GeneralSettings />
          </div>

          <div
            ref={(el) => {
              sectionRefs.current.certificate = el;
            }}
            data-category="certificate"
            className="space-y-6"
          >
            <h2 className="text-xl font-semibold text-foreground">{categoryLabels.certificate}</h2>
            <CertificateSettings />
            <ClientCertificateSection />
            <RequestClientCertSection />
          </div>

          <div
            ref={(el) => {
              sectionRefs.current.network = el;
            }}
            data-category="network"
            className="space-y-6"
          >
            <h2 className="text-xl font-semibold text-foreground">{categoryLabels.network}</h2>
            <ThrottleSection />
            <ConnectionStrategySection />
            <SslProxyingSection />
            <UpstreamProxySection />
          </div>

          <div
            ref={(el) => {
              sectionRefs.current.security = el;
            }}
            data-category="security"
            className="space-y-6"
          >
            <h2 className="text-xl font-semibold text-foreground">{categoryLabels.security}</h2>
            <ProxyAuthSection />
          </div>

          <div
            ref={(el) => {
              sectionRefs.current.tools = el;
            }}
            data-category="tools"
            className="space-y-6"
          >
            <h2 className="text-xl font-semibold text-foreground">{categoryLabels.tools}</h2>
            <CliSettings />
            <ShortcutSection />
          </div>
        </div>
      </div>
    </form>
  );
}

// =============================================================================
// Form-based sections
// =============================================================================

function ThrottleSection() {
  const { t } = useLingui();
  const { register, watch, setValue } = useSettingsForm();
  const enabled = watch("throttle.enabled");
  const preset = watch("throttle.preset");

  return (
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
        <Switch
          checked={enabled}
          onCheckedChange={(v) => setValue("throttle.enabled", v, { shouldDirty: true })}
        />
      </div>
      {enabled && (
        <div className="space-y-4 pt-2">
          <div>
            <label className="text-sm font-medium mb-1.5 block">
              <Trans>Profile</Trans>
            </label>
            <Select
              value={preset}
              onValueChange={(v) => {
                if (v) setValue("throttle.preset", v, { shouldDirty: true });
              }}
            >
              <SelectTrigger className="w-64">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {THROTTLE_PRESETS.map((p) => (
                  <SelectItem key={p.value} value={p.value} label={p.label}>
                    {p.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {preset === "custom" && (
            <div className="flex gap-3">
              <div className="flex-1">
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>Download (KB/s)</Trans>
                </label>
                <Input
                  type="number"
                  placeholder={t`Unlimited`}
                  {...register("throttle.download")}
                />
              </div>
              <div className="flex-1">
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>Upload (KB/s)</Trans>
                </label>
                <Input type="number" placeholder={t`Unlimited`} {...register("throttle.upload")} />
              </div>
              <div className="w-28">
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>Latency (ms)</Trans>
                </label>
                <Input type="number" placeholder="0" {...register("throttle.latency")} />
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function ConnectionStrategySection() {
  const { watch, setValue } = useSettingsForm();
  const strategy = watch("connectionStrategy");

  return (
    <div className="border rounded-lg p-5 space-y-5">
      <div>
        <h2 className="text-lg font-semibold">
          <Trans>Connection Strategy</Trans>
        </h2>
        <p className="text-sm text-muted-foreground">
          <Trans>
            Controls when the proxy connects to upstream servers for certificate sniffing
          </Trans>
        </p>
      </div>
      <div className="space-y-3">
        <div>
          <label className="text-sm font-medium mb-1.5 block">
            <Trans>Strategy</Trans>
          </label>
          <Select
            value={strategy}
            onValueChange={(v) => {
              if (v) setValue("connectionStrategy", v as ConnectionStrategy, { shouldDirty: true });
            }}
          >
            <SelectTrigger className="w-72">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {STRATEGY_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value} label={opt.label}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <p className="text-xs text-muted-foreground">
          {strategy === "lazy" && (
            <Trans>Connects to the server only when needed (default, sequential sniffing)</Trans>
          )}
          {strategy === "eager" && (
            <Trans>
              Starts background server connection immediately after ClientHello detection
            </Trans>
          )}
          {strategy === "eager_with_fallback" && (
            <Trans>Tries Eager first, falls back to Lazy on failure</Trans>
          )}
        </p>
      </div>
    </div>
  );
}

function UpstreamProxySection() {
  const { t } = useLingui();
  const { register, watch, setValue } = useSettingsForm();
  const enabled = watch("upstreamProxy.enabled");
  const useAuth = watch("upstreamProxy.useAuth");

  return (
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
        <Switch
          checked={enabled}
          onCheckedChange={(v) => setValue("upstreamProxy.enabled", v, { shouldDirty: true })}
        />
      </div>
      {enabled && (
        <div className="space-y-4 pt-2">
          <div className="flex gap-3">
            <div className="flex-1">
              <label className="text-sm font-medium mb-1.5 block">
                <Trans>Host</Trans>
              </label>
              <Input placeholder={t`proxy.company.com`} {...register("upstreamProxy.host")} />
            </div>
            <div className="w-28">
              <label className="text-sm font-medium mb-1.5 block">
                <Trans>Port</Trans>
              </label>
              <Input type="number" placeholder="8080" {...register("upstreamProxy.port")} />
            </div>
          </div>
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <Switch
                checked={useAuth}
                onCheckedChange={(v) => setValue("upstreamProxy.useAuth", v, { shouldDirty: true })}
              />
              <label className="text-sm font-medium">
                <Trans>Authentication</Trans>
              </label>
            </div>
            {useAuth && (
              <div className="flex gap-3 pl-1">
                <div className="flex-1">
                  <Input placeholder={t`Username`} {...register("upstreamProxy.username")} />
                </div>
                <div className="flex-1">
                  <Input
                    type="password"
                    placeholder={t`Password`}
                    {...register("upstreamProxy.password")}
                  />
                </div>
              </div>
            )}
          </div>
          <div>
            <label className="text-sm font-medium mb-1.5 block">
              <Trans>Bypass List</Trans>
            </label>
            <Input
              placeholder={t`localhost, 127.0.0.1, *.internal.com`}
              {...register("upstreamProxy.bypass")}
            />
            <p className="text-xs text-muted-foreground mt-1">
              <Trans>
                Comma-separated list of hosts to connect directly (supports *.domain.com wildcards)
              </Trans>
            </p>
          </div>
        </div>
      )}
    </div>
  );
}

function ProxyAuthSection() {
  const { t } = useLingui();
  const { register, watch, setValue } = useSettingsForm();
  const enabled = watch("proxyAuth.enabled");

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
        <Switch
          checked={enabled}
          onCheckedChange={(v) => setValue("proxyAuth.enabled", v, { shouldDirty: true })}
        />
      </div>
      {enabled && (
        <div className="space-y-4 pt-2">
          <div className="flex gap-3">
            <div className="flex-1">
              <label className="text-sm font-medium mb-1.5 block">
                <Trans>Username</Trans>
              </label>
              <Input placeholder={t`Username`} {...register("proxyAuth.username")} />
            </div>
            <div className="flex-1">
              <label className="text-sm font-medium mb-1.5 block">
                <Trans>Password</Trans>
              </label>
              <Input
                type="password"
                placeholder={t`Password`}
                {...register("proxyAuth.password")}
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
    </div>
  );
}

// --- Shortcut ---
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

function ShortcutSection() {
  const { t } = useLingui();
  const { watch, setValue } = useSettingsForm();
  const enabled = watch("shortcut.enabled");
  const hotkey = watch("shortcut.key");
  const [isRecording, setIsRecording] = useState(false);

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
      if (["Control", "Meta", "Alt", "Shift"].includes(key)) return;
      if (!hasCtrlOrCmd && !hasAlt) return;
      let keyName = key;
      if (key.length === 1) keyName = key.toUpperCase();
      else if (key === " ") keyName = "Space";
      parts.push(keyName);
      setValue("shortcut.key", parts.join("+"), { shouldDirty: true });
      setIsRecording(false);
    },
    [isRecording, setValue],
  );

  return (
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
          checked={enabled}
          onCheckedChange={(v) => setValue("shortcut.enabled", v, { shouldDirty: true })}
        />
      </div>
      {enabled && (
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
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => setIsRecording(!isRecording)}
              >
                {isRecording ? t`Cancel` : t`Change`}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// --- Client Certificate ---
function ClientCertificateSection() {
  const { t } = useLingui();
  const { watch, setValue } = useSettingsForm();
  const enabled = watch("clientCert.enabled");
  const certPath = watch("clientCert.certPath");
  const keyPath = watch("clientCert.keyPath");
  const domainCerts = watch("clientCert.domainCerts");

  const [certInfo, setCertInfo] = useState<CertificateInfo | null>(null);
  const [certInfoLoading, setCertInfoLoading] = useState(false);
  const [newDomainPattern, setNewDomainPattern] = useState("");
  const [newDomainCertPath, setNewDomainCertPath] = useState("");
  const [newDomainKeyPath, setNewDomainKeyPath] = useState("");

  const loadCertInfo = useCallback(async (path: string) => {
    setCertInfoLoading(true);
    try {
      setCertInfo(await parseCertificateInfo(path));
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
      setValue("clientCert.certPath", path, { shouldDirty: true });
      loadCertInfo(path);
    }
  }, [setValue, loadCertInfo]);

  const handleSelectKey = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Key", extensions: ["pem", "key"] }],
    });
    if (selected) {
      setValue("clientCert.keyPath", selected as string, { shouldDirty: true });
    }
  }, [setValue]);

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
    if (domainCerts.some((dc) => dc.domain_pattern === pattern)) return;
    setValue(
      "clientCert.domainCerts",
      [
        ...domainCerts,
        {
          domain_pattern: pattern,
          cert_path: newDomainCertPath,
          key_path: newDomainKeyPath,
          enabled: true,
        },
      ],
      { shouldDirty: true },
    );
    setNewDomainPattern("");
    setNewDomainCertPath("");
    setNewDomainKeyPath("");
  }, [newDomainPattern, newDomainCertPath, newDomainKeyPath, domainCerts, setValue]);

  const toggleDomainCert = useCallback(
    (idx: number) => {
      setValue(
        "clientCert.domainCerts",
        domainCerts.map((dc, i) => (i === idx ? { ...dc, enabled: !dc.enabled } : dc)),
        { shouldDirty: true },
      );
    },
    [domainCerts, setValue],
  );

  const removeDomainCert = useCallback(
    (idx: number) => {
      setValue(
        "clientCert.domainCerts",
        domainCerts.filter((_, i) => i !== idx),
        { shouldDirty: true },
      );
    },
    [domainCerts, setValue],
  );

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
        <Switch
          checked={enabled}
          onCheckedChange={(v) => setValue("clientCert.enabled", v, { shouldDirty: true })}
        />
      </div>

      {enabled && (
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
              <Button
                type="button"
                variant="outline"
                onClick={handleSelectCert}
              >{t`Browse`}</Button>
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
              <Button type="button" variant="outline" onClick={handleSelectKey}>{t`Browse`}</Button>
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

          {/* Domain-specific Certificates */}
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
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleSelectDomainCert}
                >{t`Browse`}</Button>
              </div>
              <div className="flex gap-2">
                <Input
                  readOnly
                  placeholder={t`Key file`}
                  value={newDomainKeyPath}
                  className="flex-1"
                />
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleSelectDomainKey}
                >{t`Browse`}</Button>
              </div>
              <Button
                type="button"
                onClick={handleAddDomainCert}
                disabled={!newDomainPattern.trim() || !newDomainCertPath || !newDomainKeyPath}
              >
                <Trans>Add Domain Certificate</Trans>
              </Button>
            </div>
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
                      type="button"
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
    </div>
  );
}

// --- Request Client Cert ---
function RequestClientCertSection() {
  const { t } = useLingui();
  const { watch, setValue } = useSettingsForm();
  const enabled = watch("requestClientCert.enabled");
  const caCertPath = watch("requestClientCert.caCertPath");
  const required = watch("requestClientCert.required");

  const handleSelectCaCert = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Certificate", extensions: ["pem", "crt", "cer"] }],
    });
    if (selected) {
      setValue("requestClientCert.caCertPath", selected as string, { shouldDirty: true });
    }
  }, [setValue]);

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
        <Switch
          checked={enabled}
          onCheckedChange={(v) => setValue("requestClientCert.enabled", v, { shouldDirty: true })}
        />
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
              <Button
                type="button"
                variant="outline"
                onClick={handleSelectCaCert}
              >{t`Browse`}</Button>
              {caCertPath && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() =>
                    setValue("requestClientCert.caCertPath", "", { shouldDirty: true })
                  }
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
            <Switch
              checked={required}
              onCheckedChange={(v) =>
                setValue("requestClientCert.required", v, { shouldDirty: true })
              }
            />
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
    </div>
  );
}

// --- SSL Proxying (form-integrated, saved via Save button) ---
function SslProxyingSection() {
  const { t } = useLingui();
  const form = useSettingsForm();
  const mode = form.watch("sslProxying.mode");
  const entries = form.watch("sslProxying.entries");
  const [newPattern, setNewPattern] = useState("");

  const handleAdd = useCallback(() => {
    const pattern = newPattern.trim();
    if (!pattern || entries.some((e) => e.pattern === pattern)) return;
    form.setValue("sslProxying.entries", [...entries, { pattern, enabled: true }], {
      shouldDirty: true,
    });
    setNewPattern("");
  }, [newPattern, entries, form]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAdd();
      }
    },
    [handleAdd],
  );

  const handleRemove = useCallback(
    (pattern: string) => {
      form.setValue(
        "sslProxying.entries",
        entries.filter((e) => e.pattern !== pattern),
        { shouldDirty: true },
      );
    },
    [entries, form],
  );

  const handleToggle = useCallback(
    (pattern: string) => {
      form.setValue(
        "sslProxying.entries",
        entries.map((e) => (e.pattern === pattern ? { ...e, enabled: !e.enabled } : e)),
        { shouldDirty: true },
      );
    },
    [entries, form],
  );

  const handleModeChange = useCallback(
    (v: string) => {
      form.setValue("sslProxying.mode", v as "blacklist" | "whitelist", { shouldDirty: true });
    },
    [form],
  );

  const enabledCount = entries.filter((e) => e.enabled).length;

  return (
    <div className="border rounded-lg p-5 space-y-4">
      <div>
        <h2 className="text-lg font-semibold">
          <Trans>SSL Proxying</Trans>
        </h2>
        <p className="text-sm text-muted-foreground">
          {mode === "blacklist" ? (
            enabledCount === 0 ? (
              <Trans>
                All HTTPS traffic is intercepted. OAuth domains (accounts.google.com, etc.) are
                automatically excluded.
              </Trans>
            ) : (
              <Trans>
                All HTTPS traffic is intercepted except {enabledCount} excluded domain(s) and
                built-in OAuth domains.
              </Trans>
            )
          ) : enabledCount === 0 ? (
            <Trans>All HTTPS traffic is being intercepted (no whitelist configured)</Trans>
          ) : (
            <Trans>
              Only whitelisted domains ({enabledCount}) will have HTTPS traffic intercepted
            </Trans>
          )}
        </p>
      </div>
      <div className="flex items-center gap-3">
        <span className="text-sm font-medium whitespace-nowrap">
          <Trans>Mode</Trans>
        </span>
        <Select
          value={mode}
          onValueChange={(v) => {
            if (v) handleModeChange(v);
          }}
        >
          <SelectTrigger className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="blacklist">
              <Trans>Blacklist (Exclude)</Trans>
            </SelectItem>
            <SelectItem value="whitelist">
              <Trans>Whitelist (Include)</Trans>
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div className="flex items-center gap-2">
        <Input
          placeholder={t`example.com, *.example.com, or example.com:443`}
          value={newPattern}
          onChange={(e) => setNewPattern(e.target.value)}
          onKeyDown={handleKeyDown}
          className="flex-1"
        />
        <Button type="button" onClick={handleAdd} disabled={!newPattern.trim()}>
          <Trans>Add</Trans>
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">
        {mode === "blacklist" ? (
          <Trans>
            Domains in this list will NOT be intercepted (pass-through). OAuth domains like
            accounts.google.com are always excluded by default.
          </Trans>
        ) : (
          <Trans>
            Only domains in this list will be intercepted. When the list is empty, all domains are
            intercepted.
          </Trans>
        )}
      </p>
      {entries.length > 0 && (
        <div className="border rounded-lg divide-y">
          {entries.map((entry) => (
            <div key={entry.pattern} className="flex items-center justify-between px-4 py-2">
              <div className="flex items-center gap-3">
                <Switch
                  checked={entry.enabled}
                  onCheckedChange={() => handleToggle(entry.pattern)}
                />
                <span
                  className={`font-mono text-sm ${entry.enabled ? "text-foreground" : "text-muted-foreground line-through"}`}
                >
                  {entry.pattern}
                </span>
              </div>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => handleRemove(entry.pattern)}
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
