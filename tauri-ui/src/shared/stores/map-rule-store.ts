import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { InterceptRule } from "@/entities/intercept-rule";
import { updateInterceptRules } from "@/shared/api/proxy";
import { useInterceptRuleStore } from "./intercept-rule-store";

export interface MapRuleStoreState {
  rules: InterceptRule[];
  addRule: (rule: InterceptRule) => void;
  updateRule: (rule: InterceptRule) => void;
  removeRule: (id: string) => void;
  toggleRule: (id: string) => void;
  clearRules: () => void;
  syncToProxy: () => Promise<void>;
}

export const useMapRuleStore = create<MapRuleStoreState>()(
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
          // Map rules와 Intercept rules를 합쳐서 전송
          const mapRules = get().rules;
          const interceptRules = useInterceptRuleStore.getState().rules;
          await updateInterceptRules([...interceptRules, ...mapRules]);
        } catch (error) {
          console.error("Failed to sync map rules:", error);
        }
      },
    }),
    {
      name: "cheolsu-map-rules",
    },
  ),
);
