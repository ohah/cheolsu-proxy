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
