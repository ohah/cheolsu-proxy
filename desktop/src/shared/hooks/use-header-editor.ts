import { useState, useCallback } from "react";

export interface HeaderEntry {
  key: string;
  value: string;
}

export function headersToEntries(headers: Record<string, string>): HeaderEntry[] {
  return Object.entries(headers).map(([key, value]) => ({ key, value }));
}

export function entriesToHeaders(entries: HeaderEntry[]): Record<string, string> {
  const headers: Record<string, string> = {};
  for (const { key, value } of entries) {
    const k = key.trim();
    if (k) headers[k] = value;
  }
  return headers;
}

interface UseHeaderEditorOptions {
  initialHeaders?: HeaderEntry[];
}

export function useHeaderEditor(options?: UseHeaderEditorOptions) {
  const [headers, setHeaders] = useState<HeaderEntry[]>(options?.initialHeaders ?? []);

  const addHeader = useCallback(() => {
    setHeaders((prev) => [...prev, { key: "", value: "" }]);
  }, []);

  const removeHeader = useCallback((index: number) => {
    setHeaders((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const updateHeader = useCallback((index: number, field: "key" | "value", value: string) => {
    setHeaders((prev) => {
      const updated = [...prev];
      updated[index] = { ...updated[index], [field]: value };
      return updated;
    });
  }, []);

  const resetHeaders = useCallback((newHeaders: HeaderEntry[]) => {
    setHeaders(newHeaders);
  }, []);

  return {
    headers,
    setHeaders,
    addHeader,
    removeHeader,
    updateHeader,
    resetHeaders,
  };
}
