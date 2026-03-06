import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { InterceptRule, InterceptRuleStoreState } from "@/entities/intercept-rule";
import { registerRuleStore, syncAllRulesToProxy } from "./sync-rules";

export const useInterceptRuleStore = create<InterceptRuleStoreState>()(
  persist(
    (set, get) => ({
      rules: [],

      addRule: (rule: InterceptRule) => {
        set((state) => ({ rules: [...state.rules, rule] }));
        get().syncToProxy();
      },

      updateRule: (rule: InterceptRule) => {
        set((state) => ({
          rules: state.rules.map((r) => (r.id === rule.id ? rule : r)),
        }));
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

      clearRules: () => {
        set({ rules: [] });
        get().syncToProxy();
      },

      syncToProxy: async () => {
        try {
          await syncAllRulesToProxy();
        } catch (error) {
          console.error("Failed to sync intercept rules:", error);
        }
      },
    }),
    {
      name: "cheolsu-intercept-rules",
    },
  ),
);

registerRuleStore(() => useInterceptRuleStore.getState());
