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
  type ThrottleConfig,
} from "@/shared/api/proxy";
import {
  setStoredShortcut,
  setShortcutEnabled,
  registerShortcut,
  unregisterShortcut,
} from "@/shared/lib/global-shortcut";
import { toggleProxy } from "@/features/proxy-toggle";
import type { SettingsFormValues } from "./settings-form";
import { THROTTLE_PRESETS } from "./constants";

// =============================================================================
// Save logic — only saves dirty sections, collects errors per-section
//
// Note: This function is called outside of React components, so we use
// zustand's `getState()` API to access store state directly.
// This is an officially supported pattern by zustand for non-React contexts.
// See: https://docs.pmnd.rs/zustand/guides/practice-with-no-store-actions
// =============================================================================
interface SaveResult {
  section: string;
  success: boolean;
  error?: unknown;
}

export async function saveAllSettings(
  data: SettingsFormValues,
  dirtyFields: Partial<Record<keyof SettingsFormValues, unknown>>,
) {
  const store = useAppSettingsStore.getState();
  const results: SaveResult[] = [];

  // Proxy Port
  if (dirtyFields.proxyPort) {
    try {
      store.setProxyPort(data.proxyPort);
      results.push({ section: "proxyPort", success: true });
    } catch (error) {
      results.push({ section: "proxyPort", success: false, error });
    }
  }

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
              ? {
                  username: data.upstreamProxy.username,
                  password: data.upstreamProxy.password,
                }
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
      sslStore.setDefaultPassthroughEntries(data.sslProxying.defaultPassthroughEntries);
      await sslStore.syncToProxy();
      await sslStore.syncDefaultPassthroughToProxy();
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
