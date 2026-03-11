import { createContext, useContext, useCallback, useState, useRef, type ReactNode } from "react";

type SaveFn = () => Promise<void>;

interface SettingsSaveContextValue {
  registerSave: (id: string, saveFn: SaveFn) => void;
  unregisterSave: (id: string) => void;
  markDirty: (id: string) => void;
  markClean: (id: string) => void;
  isDirty: boolean;
  saveAll: () => Promise<{ success: boolean; errors: string[] }>;
  isSaving: boolean;
}

const SettingsSaveContext = createContext<SettingsSaveContextValue | null>(null);

export function SettingsSaveProvider({ children }: { children: ReactNode }) {
  const saveFns = useRef(new Map<string, SaveFn>());
  const [dirtySet, setDirtySet] = useState(new Set<string>());
  const [isSaving, setIsSaving] = useState(false);

  const registerSave = useCallback((id: string, saveFn: SaveFn) => {
    saveFns.current.set(id, saveFn);
  }, []);

  const unregisterSave = useCallback((id: string) => {
    saveFns.current.delete(id);
    setDirtySet((prev) => {
      if (!prev.has(id)) return prev;
      const next = new Set(prev);
      next.delete(id);
      return next;
    });
  }, []);

  const markDirty = useCallback((id: string) => {
    setDirtySet((prev) => {
      if (prev.has(id)) return prev;
      const next = new Set(prev);
      next.add(id);
      return next;
    });
  }, []);

  const markClean = useCallback((id: string) => {
    setDirtySet((prev) => {
      if (!prev.has(id)) return prev;
      const next = new Set(prev);
      next.delete(id);
      return next;
    });
  }, []);

  const saveAll = useCallback(async () => {
    setIsSaving(true);
    const errors: string[] = [];
    const dirtyIds = [...dirtySet];

    for (const id of dirtyIds) {
      const saveFn = saveFns.current.get(id);
      if (saveFn) {
        try {
          await saveFn();
        } catch {
          errors.push(id);
        }
      }
    }

    setDirtySet((prev) => {
      const next = new Set(prev);
      for (const id of dirtyIds) {
        if (!errors.includes(id)) {
          next.delete(id);
        }
      }
      return next;
    });

    setIsSaving(false);
    return { success: errors.length === 0, errors };
  }, [dirtySet]);

  return (
    <SettingsSaveContext.Provider
      value={{
        registerSave,
        unregisterSave,
        markDirty,
        markClean,
        isDirty: dirtySet.size > 0,
        saveAll,
        isSaving,
      }}
    >
      {children}
    </SettingsSaveContext.Provider>
  );
}

export function useSettingsSave() {
  const ctx = useContext(SettingsSaveContext);
  if (!ctx) throw new Error("useSettingsSave must be used within SettingsSaveProvider");
  return ctx;
}
