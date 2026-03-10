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
    },
  ),
);
