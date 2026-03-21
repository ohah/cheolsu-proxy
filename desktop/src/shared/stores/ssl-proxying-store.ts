import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { createTauriStorage } from "@/shared/lib/tauri-store-storage";
import type { SslProxyingEntry, SslProxyingMode } from "@/shared/api/proxy";
import {
  updateSslProxyingList,
  updateDefaultPassthroughDomains,
  getDefaultPassthroughDomains,
} from "@/shared/api/proxy";
import { createDebouncedSync } from "./create-debounced-sync";

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
  /** 데몬 이벤트로 수신한 기본 패스스루 도메인 반영 전용 — syncDefaultPassthroughToProxy 호출 안 함 */
  setDefaultPassthroughFromDaemon: (entries: SslProxyingEntry[]) => void;
  /** 기본 패스스루 도메인 목록 설정 및 데몬 동기화 */
  setDefaultPassthroughEntries: (entries: SslProxyingEntry[]) => void;
  /** 기본 패스스루 도메인을 프록시에 동기화 */
  syncDefaultPassthroughToProxy: () => void;
  /** Rust 백엔드에서 기본값을 가져와 기본 패스스루 도메인을 복원 */
  restoreDefaultPassthrough: () => Promise<SslProxyingEntry[]>;
}

/** entries 동기화용 debounce */
const debouncedSyncEntries = createDebouncedSync();
/** defaultPassthrough 동기화용 debounce (별도 API 엔드포인트) */
const debouncedSyncPassthrough = createDebouncedSync();

export const useSslProxyingStore = create<SslProxyingStoreState>()(
  persist(
    (set, get) => ({
      mode: "blacklist" as SslProxyingMode,
      entries: [],
      defaultPassthroughEntries: [],

      setMode: (mode: SslProxyingMode) => {
        set({ mode });
        get().syncToProxy();
      },

      addEntry: (entry: SslProxyingEntry) => {
        set((state) => ({ entries: [...state.entries, entry] }));
        get().syncToProxy();
      },

      removeEntry: (pattern: string) => {
        set((state) => ({
          entries: state.entries.filter((e) => e.pattern !== pattern),
        }));
        get().syncToProxy();
      },

      toggleEntry: (pattern: string) => {
        set((state) => ({
          entries: state.entries.map((e) =>
            e.pattern === pattern ? { ...e, enabled: !e.enabled } : e,
          ),
        }));
        get().syncToProxy();
      },

      /** 데몬 이벤트로 수신한 상태 반영 전용 -- syncToProxy 호출 안 함 */
      setFromDaemon: (mode: SslProxyingMode, entries: SslProxyingEntry[]) => {
        set({ mode, entries });
      },

      clearEntries: () => {
        set({ entries: [] });
        get().syncToProxy();
      },

      syncToProxy: () => {
        debouncedSyncEntries(async () => {
          try {
            const { mode, entries } = get();
            await updateSslProxyingList(mode, entries);
          } catch (error) {
            console.error("Failed to sync SSL proxying list:", error);
          }
        });
      },

      /** 데몬 이벤트로 수신한 기본 패스스루 도메인 반영 전용 — sync 호출 안 함 */
      setDefaultPassthroughFromDaemon: (entries: SslProxyingEntry[]) => {
        set({ defaultPassthroughEntries: entries });
      },

      setDefaultPassthroughEntries: (entries: SslProxyingEntry[]) => {
        set({ defaultPassthroughEntries: entries });
        get().syncDefaultPassthroughToProxy();
      },

      syncDefaultPassthroughToProxy: () => {
        debouncedSyncPassthrough(async () => {
          try {
            const { defaultPassthroughEntries } = get();
            await updateDefaultPassthroughDomains(defaultPassthroughEntries);
          } catch (error) {
            console.error("Failed to sync default passthrough domains:", error);
          }
        });
      },

      restoreDefaultPassthrough: async () => {
        const defaults = await getDefaultPassthroughDomains();
        set({ defaultPassthroughEntries: defaults });
        get().syncDefaultPassthroughToProxy();
        return defaults;
      },
    }),
    {
      name: "cheolsu-ssl-proxying",
      storage: createJSONStorage(() => createTauriStorage()),
      onRehydrateStorage: () => (state) => {
        if (!state) return;
        // entries 동기화
        state.syncToProxy();
        // defaultPassthrough: persist에서 복원된 값이 있으면 동기화, 없으면 백엔드 기본값 로딩
        if (state.defaultPassthroughEntries.length > 0) {
          state.syncDefaultPassthroughToProxy();
        } else {
          getDefaultPassthroughDomains().then((defaults) => {
            state.setDefaultPassthroughEntries(defaults);
          });
        }
      },
    },
  ),
);
