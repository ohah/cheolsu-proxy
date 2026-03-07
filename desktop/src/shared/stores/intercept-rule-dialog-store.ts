import { create } from "zustand";

export interface InterceptRuleInitialValues {
  pattern: string;
  method: string | null;
}

interface InterceptRuleDialogStore {
  open: boolean;
  initialValues: InterceptRuleInitialValues | null;
  openWithValues: (values: InterceptRuleInitialValues) => void;
  close: () => void;
}

export const useInterceptRuleDialogStore = create<InterceptRuleDialogStore>((set) => ({
  open: false,
  initialValues: null,
  openWithValues: (values) => set({ open: true, initialValues: values }),
  close: () => set({ open: false, initialValues: null }),
}));
