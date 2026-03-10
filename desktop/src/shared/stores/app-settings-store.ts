import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { Locale } from "@/shared/lib/i18n";

interface ThrottleConfigState {
  enabled: boolean;
  preset: string;
  download: string;
  upload: string;
  latency: string;
}

interface UpstreamProxyConfigState {
  enabled: boolean;
  host: string;
  port: number;
  auth: { username: string; password: string } | null;
  bypass: string[];
}

interface ProxyAuthConfigState {
  enabled: boolean;
  username: string;
  password: string;
}

interface AppSettingsState {
  // Locale
  locale: Locale;
  setLocale: (locale: Locale) => void;

  // Autosave session
  autosaveSession: boolean;
  setAutosaveSession: (enabled: boolean) => void;

  // Global shortcut
  proxyToggleShortcut: string;
  setProxyToggleShortcut: (shortcut: string) => void;
  proxyToggleShortcutEnabled: boolean;
  setProxyToggleShortcutEnabled: (enabled: boolean) => void;

  // Quick settings
  quickSettingsNoCaching: boolean;
  setQuickSettingsNoCaching: (enabled: boolean) => void;
  quickSettingsBlockCookies: boolean;
  setQuickSettingsBlockCookies: (enabled: boolean) => void;
  quickSettingsNoGzip: boolean;
  setQuickSettingsNoGzip: (enabled: boolean) => void;
  setQuickSettings: (
    settings: Partial<
      Pick<
        AppSettingsState,
        "quickSettingsNoCaching" | "quickSettingsBlockCookies" | "quickSettingsNoGzip"
      >
    >,
  ) => void;

  // Throttle config
  throttleConfig: ThrottleConfigState;
  setThrottleConfig: (config: ThrottleConfigState) => void;

  // Upstream proxy config
  upstreamProxyConfig: UpstreamProxyConfigState;
  setUpstreamProxyConfig: (config: UpstreamProxyConfigState) => void;

  // Proxy auth config
  proxyAuthConfig: ProxyAuthConfigState;
  setProxyAuthConfig: (config: ProxyAuthConfigState) => void;
}

/**
 * 기존 localStorage 개별 키에서 데이터를 마이그레이션하여
 * Zustand persist store의 초기 상태로 사용합니다.
 */
function migrateFromLegacyLocalStorage(): Partial<AppSettingsState> {
  const migrated: Partial<AppSettingsState> = {};
  const LEGACY_KEYS = [
    "locale",
    "autosave_session",
    "proxy_toggle_shortcut",
    "proxy_toggle_shortcut_enabled",
    "quick_settings_no_caching",
    "quick_settings_block_cookies",
    "quick_settings_no_gzip",
    "throttle_config",
    "upstream_proxy_config",
    "proxy_auth_config",
  ];

  try {
    const locale = localStorage.getItem("locale");
    if (locale) migrated.locale = locale as Locale;

    const autosave = localStorage.getItem("autosave_session");
    if (autosave !== null) migrated.autosaveSession = autosave !== "false";

    const shortcut = localStorage.getItem("proxy_toggle_shortcut");
    if (shortcut) migrated.proxyToggleShortcut = shortcut;

    const shortcutEnabled = localStorage.getItem("proxy_toggle_shortcut_enabled");
    if (shortcutEnabled !== null) migrated.proxyToggleShortcutEnabled = shortcutEnabled === "true";

    const noCaching = localStorage.getItem("quick_settings_no_caching");
    if (noCaching !== null) {
      try {
        migrated.quickSettingsNoCaching = JSON.parse(noCaching);
      } catch {
        /* ignore */
      }
    }

    const blockCookies = localStorage.getItem("quick_settings_block_cookies");
    if (blockCookies !== null) {
      try {
        migrated.quickSettingsBlockCookies = JSON.parse(blockCookies);
      } catch {
        /* ignore */
      }
    }

    const noGzip = localStorage.getItem("quick_settings_no_gzip");
    if (noGzip !== null) {
      try {
        migrated.quickSettingsNoGzip = JSON.parse(noGzip);
      } catch {
        /* ignore */
      }
    }

    const throttle = localStorage.getItem("throttle_config");
    if (throttle) {
      try {
        const parsed = JSON.parse(throttle);
        migrated.throttleConfig = {
          enabled: parsed.enabled ?? false,
          preset: parsed.preset ?? "none",
          download: parsed.download ?? "",
          upload: parsed.upload ?? "",
          latency: String(parsed.latency ?? "0"),
        };
      } catch {
        /* ignore */
      }
    }

    const upstream = localStorage.getItem("upstream_proxy_config");
    if (upstream) {
      try {
        const parsed = JSON.parse(upstream);
        migrated.upstreamProxyConfig = {
          enabled: parsed.enabled ?? false,
          host: parsed.host ?? "",
          port: parsed.port ?? 8080,
          auth: parsed.auth ?? null,
          bypass: parsed.bypass ?? [],
        };
      } catch {
        /* ignore */
      }
    }

    const proxyAuth = localStorage.getItem("proxy_auth_config");
    if (proxyAuth) {
      try {
        const parsed = JSON.parse(proxyAuth);
        migrated.proxyAuthConfig = {
          enabled: parsed.enabled ?? false,
          username: parsed.username ?? "",
          password: parsed.password ?? "",
        };
      } catch {
        /* ignore */
      }
    }

    // 마이그레이션 완료 후 레거시 키 정리
    LEGACY_KEYS.forEach((key) => localStorage.removeItem(key));
  } catch {
    /* localStorage 접근 실패 시 무시 */
  }

  return migrated;
}

const DEFAULT_THROTTLE_CONFIG: ThrottleConfigState = {
  enabled: false,
  preset: "none",
  download: "",
  upload: "",
  latency: "0",
};

const DEFAULT_UPSTREAM_PROXY_CONFIG: UpstreamProxyConfigState = {
  enabled: false,
  host: "",
  port: 8080,
  auth: null,
  bypass: [],
};

const DEFAULT_PROXY_AUTH_CONFIG: ProxyAuthConfigState = {
  enabled: false,
  username: "",
  password: "",
};

export const useAppSettingsStore = create<AppSettingsState>()(
  persist(
    (set) => ({
      locale: "en" as Locale,
      setLocale: (locale) => set({ locale }),

      autosaveSession: true,
      setAutosaveSession: (enabled) => set({ autosaveSession: enabled }),

      proxyToggleShortcut: "CommandOrControl+Shift+P",
      setProxyToggleShortcut: (shortcut) => set({ proxyToggleShortcut: shortcut }),
      proxyToggleShortcutEnabled: true,
      setProxyToggleShortcutEnabled: (enabled) => set({ proxyToggleShortcutEnabled: enabled }),

      quickSettingsNoCaching: false,
      setQuickSettingsNoCaching: (enabled) => set({ quickSettingsNoCaching: enabled }),
      quickSettingsBlockCookies: false,
      setQuickSettingsBlockCookies: (enabled) => set({ quickSettingsBlockCookies: enabled }),
      quickSettingsNoGzip: false,
      setQuickSettingsNoGzip: (enabled) => set({ quickSettingsNoGzip: enabled }),
      // 그룹 setter
      setQuickSettings: (settings) => set(settings),

      throttleConfig: DEFAULT_THROTTLE_CONFIG,
      setThrottleConfig: (config) => set({ throttleConfig: config }),

      upstreamProxyConfig: DEFAULT_UPSTREAM_PROXY_CONFIG,
      setUpstreamProxyConfig: (config) => set({ upstreamProxyConfig: config }),

      proxyAuthConfig: DEFAULT_PROXY_AUTH_CONFIG,
      setProxyAuthConfig: (config) => set({ proxyAuthConfig: config }),
    }),
    {
      name: "cheolsu-app-settings",
      version: 1,
      migrate: (_persistedState, version) => {
        if (version === 0) {
          // 신규 사용자: persistedState가 없으므로 version === 0으로 진입
          // → 기존 localStorage 개별 키에서 데이터를 자동 마이그레이션
          const legacy = migrateFromLegacyLocalStorage();
          return {
            ...(_persistedState as AppSettingsState),
            ...legacy,
          };
        }
        return _persistedState as AppSettingsState;
      },
    },
  ),
);
