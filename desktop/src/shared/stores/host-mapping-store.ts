import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { HostMapping } from "@/shared/api/proxy";
import { updateHostMappings } from "@/shared/api/proxy";

interface HostMappingStoreState {
  hostMappings: HostMapping[];
  addMapping: (mapping: HostMapping) => void;
  removeMapping: (id: string) => void;
  toggleMapping: (id: string) => void;
  setMappings: (mappings: HostMapping[]) => void;
  clearMappings: () => void;
  syncToProxy: () => Promise<void>;
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

      setMappings: (mappings: HostMapping[]) => {
        set({ hostMappings: mappings });
      },

      clearMappings: () => {
        set({ hostMappings: [] });
        get().syncToProxy();
      },

      syncToProxy: async () => {
        try {
          const { hostMappings } = get();
          await updateHostMappings(hostMappings);
        } catch (error) {
          console.error("Failed to sync host mappings:", error);
        }
      },
    }),
    {
      name: "cheolsu-host-mappings",
    },
  ),
);
