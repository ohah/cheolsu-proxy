import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { Locale } from "@/shared/lib/i18n";
import { getStoredShortcut, getShortcutEnabled } from "@/shared/lib/global-shortcut";

interface ThrottleLocalConfig {
  enabled: boolean;
  preset: string;
  download: string;
  upload: string;
  latency: string;
}

interface SettingsState {
  // Locale
  locale: Locale;
  setLocale: (locale: Locale) => void;

  // Autosave session
  autosaveEnabled: boolean;
  setAutosaveEnabled: (enabled: boolean) => void;

  // Quick settings
  noCaching: boolean;
  setNoCaching: (enabled: boolean) => void;
  blockCookies: boolean;
  setBlockCookies: (enabled: boolean) => void;
  noGzip: boolean;
  setNoGzip: (enabled: boolean) => void;

  // Throttle config (local UI state)
  throttleConfig: ThrottleLocalConfig;
  setThrottleConfig: (config: ThrottleLocalConfig) => void;

  // Hotkey
  hotkeyEnabled: boolean;
  setHotkeyEnabled: (enabled: boolean) => void;
  hotkey: string;
  setHotkey: (hotkey: string) => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      // Locale - 기존 localStorage 키 "locale"에서 마이그레이션
      locale: (localStorage.getItem("locale") as Locale) || "en",
      setLocale: (locale: Locale) => set({ locale }),

      // Autosave session - 기존 localStorage 키 "autosave_session"에서 마이그레이션
      autosaveEnabled: localStorage.getItem("autosave_session") !== "false",
      setAutosaveEnabled: (enabled: boolean) => set({ autosaveEnabled: enabled }),

      // Quick settings - 기존 localStorage 키에서 마이그레이션
      noCaching: (() => {
        try {
          return JSON.parse(localStorage.getItem("quick_settings_no_caching") ?? "false");
        } catch {
          return false;
        }
      })(),
      setNoCaching: (enabled: boolean) => set({ noCaching: enabled }),

      blockCookies: (() => {
        try {
          return JSON.parse(localStorage.getItem("quick_settings_block_cookies") ?? "false");
        } catch {
          return false;
        }
      })(),
      setBlockCookies: (enabled: boolean) => set({ blockCookies: enabled }),

      noGzip: (() => {
        try {
          return JSON.parse(localStorage.getItem("quick_settings_no_gzip") ?? "false");
        } catch {
          return false;
        }
      })(),
      setNoGzip: (enabled: boolean) => set({ noGzip: enabled }),

      // Throttle config - 기존 localStorage 키 "throttle_config"에서 마이그레이션
      throttleConfig: (() => {
        try {
          const saved = localStorage.getItem("throttle_config");
          if (saved) {
            const parsed = JSON.parse(saved);
            return {
              enabled: parsed.enabled ?? false,
              preset: parsed.preset ?? "none",
              download: parsed.download ?? "",
              upload: parsed.upload ?? "",
              latency: String(parsed.latency ?? "0"),
            };
          }
        } catch {
          // 파싱 실패 시 무시
        }
        return { enabled: false, preset: "none", download: "", upload: "", latency: "0" };
      })(),
      setThrottleConfig: (config: ThrottleLocalConfig) => set({ throttleConfig: config }),

      // Hotkey - 기존 localStorage 함수에서 마이그레이션
      hotkeyEnabled: getShortcutEnabled(),
      setHotkeyEnabled: (enabled: boolean) => set({ hotkeyEnabled: enabled }),
      hotkey: getStoredShortcut(),
      setHotkey: (hotkey: string) => set({ hotkey }),
    }),
    {
      name: "cheolsu-settings",
      // 기존 localStorage 키와의 호환을 위해 migrate에서 처리
      migrate: (persisted, version) => {
        if (version === 0) {
          // 최초 마이그레이션 시 기존 localStorage 키 정리
          // 기존 키들은 그대로 남겨두되, 새 스토어가 우선
          return persisted as SettingsState;
        }
        return persisted as SettingsState;
      },
      version: 0,
    },
  ),
);
