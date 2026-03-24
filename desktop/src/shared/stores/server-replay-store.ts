import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { toast } from "sonner";
import { createTauriStorage } from "@/shared/lib/tauri-store-storage";
import { updateServerReplay, type ServerReplayEntry } from "@/shared/api/proxy";
import { createDebouncedSync } from "./create-debounced-sync";

interface ServerReplayStoreState {
  entries: ServerReplayEntry[];
  enabled: boolean;
  addEntry: (entry: ServerReplayEntry) => void;
  removeEntry: (id: string) => void;
  clearEntries: () => void;
  setEnabled: (enabled: boolean) => void;
  syncToProxy: () => void;
}

const debouncedSync = createDebouncedSync();

export const useServerReplayStore = create<ServerReplayStoreState>()(
  persist(
    (set, get) => ({
      entries: [],
      enabled: false,

      addEntry: (entry: ServerReplayEntry) => {
        set((state) => {
          // 같은 method+url이 이미 있으면 교체
          const filtered = state.entries.filter(
            (e) => !(e.method === entry.method && e.url === entry.url),
          );
          return { entries: [...filtered, entry] };
        });
        get().syncToProxy();
      },

      removeEntry: (id: string) => {
        set((state) => ({
          entries: state.entries.filter((e) => e.id !== id),
        }));
        get().syncToProxy();
      },

      clearEntries: () => {
        set({ entries: [] });
        get().syncToProxy();
      },

      setEnabled: (enabled: boolean) => {
        set({ enabled });
        get().syncToProxy();
      },

      syncToProxy: () => {
        debouncedSync(async () => {
          try {
            const { entries, enabled } = get();
            await updateServerReplay(enabled ? entries : []);
          } catch (error) {
            console.error("Failed to sync server replay:", error);
            toast.error("Failed to sync server replay");
          }
        });
      },
    }),
    {
      name: "cheolsu-server-replay",
      storage: createJSONStorage(() => createTauriStorage()),
      onRehydrateStorage: () => (state) => {
        if (state?.enabled && state.entries.length) {
          state.syncToProxy();
        }
      },
    },
  ),
);
