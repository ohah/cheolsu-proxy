import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { createTauriStorage } from "@/shared/lib/tauri-store-storage";
import type { InterceptRule } from "@/entities/intercept-rule";
import { registerRuleStore, syncAllRulesToProxy } from "./sync-rules";

export interface RuleStoreState {
  rules: InterceptRule[];
  addRule: (rule: InterceptRule) => void;
  updateRule: (rule: InterceptRule) => void;
  removeRule: (id: string) => void;
  toggleRule: (id: string) => void;
  clearRules: () => void;
  setRules: (rules: InterceptRule[]) => void;
  syncToProxy: () => Promise<void>;
}

interface RuleStoreConfig {
  storeName: string;
  errorLabel: string;
}

export function createRuleStore(config: RuleStoreConfig) {
  const store = create<RuleStoreState>()(
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

        setRules: (rules: InterceptRule[]) => {
          set({ rules });
        },

        syncToProxy: async () => {
          try {
            await syncAllRulesToProxy();
          } catch (error) {
            console.error(`Failed to sync ${config.errorLabel} rules:`, error);
          }
        },
      }),
      {
        name: config.storeName,
        storage: createJSONStorage(() => createTauriStorage()),
      },
    ),
  );

  registerRuleStore(() => store.getState());

  return store;
}
