import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { createTauriStorage } from "@/shared/lib/tauri-store-storage";

export interface FilterPreset {
  id: string;
  name: string;
  query: string;
}

interface FilterPresetState {
  presets: FilterPreset[];
  addPreset: (preset: Omit<FilterPreset, "id">) => void;
  updatePreset: (id: string, updates: Partial<Omit<FilterPreset, "id">>) => void;
  deletePreset: (id: string) => void;
}

export const useFilterPresetStore = create<FilterPresetState>()(
  persist(
    (set) => ({
      presets: [],

      addPreset: (preset) =>
        set((state) => ({
          presets: [...state.presets, { ...preset, id: crypto.randomUUID() }],
        })),

      updatePreset: (id, updates) =>
        set((state) => ({
          presets: state.presets.map((p) => (p.id === id ? { ...p, ...updates } : p)),
        })),

      deletePreset: (id) =>
        set((state) => ({
          presets: state.presets.filter((p) => p.id !== id),
        })),
    }),
    {
      name: "cheolsu-filter-presets",
      storage: createJSONStorage(() => createTauriStorage()),
    },
  ),
);
