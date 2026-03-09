import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { HostMapping } from "@/shared/api/proxy";
import { updateHostMappings } from "@/shared/api/proxy";

interface HostMappingStoreState {
  hostMappings: HostMapping[];
  addMapping: (mapping: HostMapping) => void;
  removeMapping: (id: string) => void;
  toggleMapping: (id: string) => void;
  /** 데몬 이벤트(host_mappings_updated)로 수신한 매핑을 반영할 때 사용.
   *  데몬에서 이미 반영된 상태이므로 syncToProxy를 호출하지 않는다. */
  setMappings: (mappings: HostMapping[]) => void;
  clearMappings: () => void;
  syncToProxy: () => void;
}

let syncTimer: ReturnType<typeof setTimeout> | null = null;

/** syncToProxy 연속 호출을 방지하기 위한 debounce (300ms) */
function debouncedSync(fn: () => Promise<void>) {
  if (syncTimer) clearTimeout(syncTimer);
  syncTimer = setTimeout(() => {
    syncTimer = null;
    fn();
  }, 300);
}

export const useHostMappingStore = create<HostMappingStoreState>()(
  persist(
    (set, get) => ({
      hostMappings: [],

      addMapping: (mapping: HostMapping) => {
        set((state) => ({ hostMappings: [...state.hostMappings, mapping] }));
        get().syncToProxy();
      },

      removeMapping: (id: string) => {
        set((state) => ({
          hostMappings: state.hostMappings.filter((m) => m.id !== id),
        }));
        get().syncToProxy();
      },

      toggleMapping: (id: string) => {
        set((state) => ({
          hostMappings: state.hostMappings.map((m) =>
            m.id === id ? { ...m, enabled: !m.enabled } : m,
          ),
        }));
        get().syncToProxy();
      },

      /** 데몬 이벤트로 수신한 매핑 반영 전용 — syncToProxy 호출 안 함 */
      setMappings: (mappings: HostMapping[]) => {
        set({ hostMappings: mappings });
      },

      clearMappings: () => {
        set({ hostMappings: [] });
        get().syncToProxy();
      },

      syncToProxy: () => {
        debouncedSync(async () => {
          try {
            const { hostMappings } = get();
            await updateHostMappings(hostMappings);
          } catch (error) {
            console.error("Failed to sync host mappings:", error);
          }
        });
      },
    }),
    {
      name: "cheolsu-host-mappings",
    },
  ),
);
