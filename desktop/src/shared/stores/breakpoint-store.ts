import { create } from "zustand";
import { persist } from "zustand/middleware";
import type {
  BreakpointRule,
  BreakpointAction,
  PendingBreakpoint,
  BreakpointStoreState,
} from "@/entities/breakpoint";
import { updateBreakpointRules, resolveBreakpoint as resolveBreakpointApi } from "@/shared/api/proxy";

export const useBreakpointStore = create<BreakpointStoreState>()(
  persist(
    (set, get) => ({
      rules: [],
      pendingBreakpoints: [],

      addRule: (rule: BreakpointRule) => {
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

      clearRules: () => {
        set({ rules: [] });
        get().syncToProxy();
      },

      setRules: (rules: BreakpointRule[]) => {
        set({ rules });
      },

      addPendingBreakpoint: (bp: PendingBreakpoint) => {
        set((state) => ({
          pendingBreakpoints: [...state.pendingBreakpoints, bp],
        }));
      },

      removePendingBreakpoint: (id: string) => {
        set((state) => ({
          pendingBreakpoints: state.pendingBreakpoints.filter((bp) => bp.id !== id),
        }));
      },

      resolveBreakpoint: async (id: string, action: BreakpointAction) => {
        try {
          await resolveBreakpointApi(id, action);
          set((state) => ({
            pendingBreakpoints: state.pendingBreakpoints.filter((bp) => bp.id !== id),
          }));
        } catch (error) {
          console.error("Failed to resolve breakpoint:", error);
        }
      },

      syncToProxy: async () => {
        try {
          const { rules } = get();
          await updateBreakpointRules(rules);
        } catch (error) {
          console.error("Failed to sync breakpoint rules:", error);
        }
      },
    }),
    {
      name: "cheolsu-breakpoint-rules",
      partialize: (state) => ({ rules: state.rules }),
    },
  ),
);
