export type ColumnKey =
  | "path"
  | "method"
  | "status"
  | "size"
  | "time"
  | "timestamp"
  | "client"
  | "waterfall";

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
  { key: "timestamp", label: "Timestamp", gridSize: "1.5fr" },
  { key: "client", label: "Client", gridSize: "1fr" },
  { key: "waterfall", label: "Waterfall", gridSize: "2fr" },
] as const;

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
  "grid items-center gap-4 px-3 py-3 border-b border-border border-l-4 border-l-transparent cursor-pointer hover:bg-muted/50 transition-colors";
export const SELECTED_ROW_CLASSES = "bg-accent/10 !border-l-accent";
export const PINNED_ROW_CLASSES = "!border-l-slate-600";
export const HEADER_CLASSES = "text-xs font-medium text-muted-foreground uppercase tracking-wide";
