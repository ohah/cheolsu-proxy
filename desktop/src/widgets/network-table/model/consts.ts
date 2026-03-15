export type ColumnKey = "path" | "method" | "status" | "size" | "time" | "client" | "waterfall";

export interface ColumnDef {
  key: ColumnKey;
  label: string;
  gridSize: string;
}

export const TABLE_COLUMNS: readonly ColumnDef[] = [
  { key: "path", label: "Path", gridSize: "5fr" },
  { key: "method", label: "Method", gridSize: "1fr" },
  { key: "status", label: "Status", gridSize: "1fr" },
  { key: "size", label: "Size", gridSize: "1fr" },
  { key: "time", label: "Time", gridSize: "1fr" },
  { key: "client", label: "Client", gridSize: "1fr" },
  { key: "waterfall", label: "Waterfall", gridSize: "2fr" },
] as const;

export const DEFAULT_VISIBLE_COLUMNS: ColumnKey[] = [
  "path",
  "method",
  "status",
  "size",
  "time",
  "waterfall",
];

const STORAGE_KEY = "network-table-visible-columns";

export function loadVisibleColumns(): Set<ColumnKey> {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored) as ColumnKey[];
      if (Array.isArray(parsed) && parsed.length > 0) {
        return new Set(parsed);
      }
    }
  } catch {
    // 파싱 실패 시 기본값 사용
  }
  return new Set(DEFAULT_VISIBLE_COLUMNS);
}

export function saveVisibleColumns(columns: Set<ColumnKey>) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify([...columns]));
}

export function buildGridTemplate(visibleColumns: Set<ColumnKey>): string {
  const sizes = ["24px"]; // checkbox
  for (const col of TABLE_COLUMNS) {
    if (visibleColumns.has(col.key)) {
      sizes.push(col.gridSize);
    }
  }
  return sizes.join(" ");
}

export const ROW_BASE_CLASSES =
  "grid gap-4 p-3 border-b border-border cursor-pointer hover:bg-muted/50 transition-colors";
export const SELECTED_ROW_CLASSES = "bg-accent/10 border-l-4 border-l-accent";
export const PINNED_ROW_CLASSES = "border-l-4 border-l-slate-600";
export const HEADER_CLASSES = "text-xs font-medium text-muted-foreground uppercase tracking-wide";
