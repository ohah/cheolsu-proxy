import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

import type { ContractSpecInfo } from "@/entities/proxy";

interface ContractStoreState {
  specs: ContractSpecInfo[];
  setSpecs: (specs: ContractSpecInfo[]) => void;
  loadSpec: (path: string) => Promise<void>;
  unloadSpec: (id: string) => Promise<void>;
  toggleSpec: (id: string, enabled: boolean) => Promise<void>;
  refreshSpecs: () => Promise<void>;
}

export const useContractStore = create<ContractStoreState>((set) => ({
  specs: [],
  setSpecs: (specs) => set({ specs }),
  loadSpec: async (path: string) => {
    await invoke("load_contract_spec", { path });
  },
  unloadSpec: async (id: string) => {
    await invoke("unload_contract_spec", { id });
  },
  toggleSpec: async (id: string, enabled: boolean) => {
    await invoke("toggle_contract_spec", { id, enabled });
  },
  refreshSpecs: async () => {
    await invoke("get_contract_specs");
  },
}));
