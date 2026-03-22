import { create } from "zustand";

export interface MapRuleInitialValues {
  pattern: string;
  method: string | null;
  mapType: "map_remote" | "map_local";
}

interface MapRuleDialogStore {
  open: boolean;
  initialValues: MapRuleInitialValues | null;
  openWithValues: (values: MapRuleInitialValues) => void;
  close: () => void;
}

export const useMapRuleDialogStore = create<MapRuleDialogStore>((set) => ({
  open: false,
  initialValues: null,
  openWithValues: (values) => set({ open: true, initialValues: values }),
  close: () => set({ open: false, initialValues: null }),
}));
