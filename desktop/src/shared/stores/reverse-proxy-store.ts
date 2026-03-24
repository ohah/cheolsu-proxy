import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { toast } from "sonner";
import { createTauriStorage } from "@/shared/lib/tauri-store-storage";
import type { ReverseProxyRule } from "@/shared/api/proxy";
import { updateReverseProxyRules } from "@/shared/api/proxy";
import { createDebouncedSync } from "./create-debounced-sync";

interface ReverseProxyStoreState {
  rules: ReverseProxyRule[];
  addRule: (rule: ReverseProxyRule) => void;
  removeRule: (id: string) => void;
  toggleRule: (id: string) => void;
  /** 데몬 이벤트(reverse_proxy_rules_updated)로 수신한 규칙을 반영할 때 사용.
   *  데몬에서 이미 반영된 상태이므로 syncToProxy를 호출하지 않는다. */
  setRules: (rules: ReverseProxyRule[]) => void;
  clearRules: () => void;
  syncToProxy: () => void;
}

const debouncedSync = createDebouncedSync();

export const useReverseProxyStore = create<ReverseProxyStoreState>()(
  persist(
    (set, get) => ({
      rules: [],

      addRule: (rule: ReverseProxyRule) => {
        set((state) => ({ rules: [...state.rules, rule] }));
        get().syncToProxy();
      },

      removeRule: (id: string) => {
        set((state) => ({
          rules: state.rules.filter((r) => r.id !== id),
        }));
        get().syncToProxy();
      },

      toggleRule: (id: string) => {
        set((state) => ({
          rules: state.rules.map((r) => (r.id === id ? { ...r, enabled: !r.enabled } : r)),
        }));
        get().syncToProxy();
      },

      setRules: (rules: ReverseProxyRule[]) => {
        set({ rules });
      },

      clearRules: () => {
        set({ rules: [] });
        get().syncToProxy();
      },

      syncToProxy: () => {
        debouncedSync(async () => {
          try {
            const { rules } = get();
            await updateReverseProxyRules(rules);
          } catch (error) {
            console.error("Failed to sync reverse proxy rules:", error);
            toast.error("Failed to sync reverse proxy rules");
          }
        });
      },
    }),
    {
      name: "cheolsu-reverse-proxy",
      storage: createJSONStorage(() => createTauriStorage()),
      onRehydrateStorage: () => (state) => {
        if (state?.rules.length) {
          state.syncToProxy();
        }
      },
    },
  ),
);
