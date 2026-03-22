import { create } from "zustand";

export interface HostMappingInitialValues {
  sourceHost: string;
  sourcePort: string;
}

interface HostMappingDialogStore {
  open: boolean;
  initialValues: HostMappingInitialValues | null;
  openWithValues: (values: HostMappingInitialValues) => void;
  openEmpty: () => void;
  close: () => void;
}

export const useHostMappingDialogStore = create<HostMappingDialogStore>((set) => ({
  open: false,
  initialValues: null,
  openWithValues: (values) => set({ open: true, initialValues: values }),
  openEmpty: () => set({ open: true, initialValues: null }),
  close: () => set({ open: false, initialValues: null }),
}));
