import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { toast } from "sonner";
import { createTauriStorage } from "@/shared/lib/tauri-store-storage";
import type { HostMapping } from "@/shared/api/proxy";
import { updateHostMappings } from "@/shared/api/proxy";
import { createDebouncedSync } from "./create-debounced-sync";

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

const debouncedSync = createDebouncedSync();

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
            toast.error("Failed to sync host mappings");
          }
        });
      },
    }),
    {
      name: "cheolsu-host-mappings",
      storage: createJSONStorage(() => createTauriStorage()),
      onRehydrateStorage: () => (state) => {
        if (state?.hostMappings.length) {
          state.syncToProxy();
        }
      },
    },
  ),
);
