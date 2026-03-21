import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { createTauriStorage } from "@/shared/lib/tauri-store-storage";
import type { SslProxyingEntry, SslProxyingMode } from "@/shared/api/proxy";
import {
  updateSslProxyingList,
  updateDefaultPassthroughDomains,
  getDefaultPassthroughDomains,
} from "@/shared/api/proxy";

interface SslProxyingStoreState {
  mode: SslProxyingMode;
  entries: SslProxyingEntry[];
  /** 기본 패스스루 도메인 목록 (사용자 커스터마이즈 가능) */
  defaultPassthroughEntries: SslProxyingEntry[];
  setMode: (mode: SslProxyingMode) => void;
  addEntry: (entry: SslProxyingEntry) => void;
  removeEntry: (pattern: string) => void;
  toggleEntry: (pattern: string) => void;
  /** 데몬 이벤트(ssl_proxying_list_updated)로 수신한 상태를 반영할 때 사용.
   *  데몬에서 이미 반영된 상태이므로 syncToProxy를 호출하지 않는다. */
  setFromDaemon: (mode: SslProxyingMode, entries: SslProxyingEntry[]) => void;
  clearEntries: () => void;
  syncToProxy: () => void;
  /** 기본 패스스루 도메인 목록 설정 (UI 및 데몬 이벤트 공용) */
  setDefaultPassthroughEntries: (entries: SslProxyingEntry[]) => void;
  /** 기본 패스스루 도메인을 프록시에 동기화 */
  syncDefaultPassthroughToProxy: () => void;
  /** Rust 백엔드에서 기본값을 가져와 기본 패스스루 도메인을 복원 */
  restoreDefaultPassthrough: () => Promise<SslProxyingEntry[]>;
  /** persist에서 복원된 값이 없을 때 Rust 백엔드에서 기본값 로딩 */
  initDefaultPassthrough: () => Promise<void>;
}

export const useSslProxyingStore = create<SslProxyingStoreState>()(
  persist(
    (set, get) => ({
      mode: "blacklist" as SslProxyingMode,
      entries: [],
      defaultPassthroughEntries: [],

      setMode: (mode: SslProxyingMode) => {
        set({ mode });
      },

      addEntry: (entry: SslProxyingEntry) => {
        set((state) => ({ entries: [...state.entries, entry] }));
      },

      removeEntry: (pattern: string) => {
        set((state) => ({
          entries: state.entries.filter((e) => e.pattern !== pattern),
        }));
      },

      toggleEntry: (pattern: string) => {
        set((state) => ({
          entries: state.entries.map((e) =>
            e.pattern === pattern ? { ...e, enabled: !e.enabled } : e,
          ),
        }));
      },

      /** 데몬 이벤트로 수신한 상태 반영 전용 -- syncToProxy 호출 안 함 */
      setFromDaemon: (mode: SslProxyingMode, entries: SslProxyingEntry[]) => {
        set({ mode, entries });
      },

      clearEntries: () => {
        set({ entries: [] });
      },

      syncToProxy: async () => {
        try {
          const { mode, entries } = get();
          await updateSslProxyingList(mode, entries);
        } catch (error) {
          console.error("Failed to sync SSL proxying list:", error);
          throw error;
        }
      },

      setDefaultPassthroughEntries: (entries: SslProxyingEntry[]) => {
        set({ defaultPassthroughEntries: entries });
      },

      syncDefaultPassthroughToProxy: async () => {
        try {
          const { defaultPassthroughEntries } = get();
          await updateDefaultPassthroughDomains(defaultPassthroughEntries);
        } catch (error) {
          console.error("Failed to sync default passthrough domains:", error);
          throw error;
        }
      },

      restoreDefaultPassthrough: async () => {
        const defaults = await getDefaultPassthroughDomains();
        set({ defaultPassthroughEntries: defaults });
        return defaults;
      },

      initDefaultPassthrough: async () => {
        const { defaultPassthroughEntries } = get();
        if (defaultPassthroughEntries.length === 0) {
          // persist에서 복원된 값이 없으면 Rust 백엔드에서 기본값 로딩
          const defaults = await getDefaultPassthroughDomains();
          set({ defaultPassthroughEntries: defaults });
        }
        // persist에서 복원된 값(비활성화 상태 포함)을 데몬에 동기화
        await get().syncDefaultPassthroughToProxy();
      },
    }),
    {
      name: "cheolsu-ssl-proxying",
      storage: createJSONStorage(() => createTauriStorage()),
    },
  ),
);
