import { create } from "zustand";

export interface ScriptLogEntry {
  level: string;
  message: string;
  time: number;
}

interface ScriptState {
  active: boolean;
  path: string | null;
  logs: ScriptLogEntry[];
  setStatus: (active: boolean, path: string | null) => void;
  addLog: (level: string, message: string) => void;
  clearLogs: () => void;
}

export const useScriptStore = create<ScriptState>()((set) => ({
  active: false,
  path: null,
  logs: [],

  setStatus: (active, path) => set({ active, path }),

  addLog: (level, message) =>
    set((state) => ({
      logs: [...state.logs, { level, message, time: Date.now() }].slice(-5000),
    })),

  clearLogs: () => set({ logs: [] }),
}));
