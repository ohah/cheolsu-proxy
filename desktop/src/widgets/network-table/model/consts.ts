export const TABLE_COLUMNS = [
  { key: "path", label: "Path" },
  { key: "method", label: "Method" },
  { key: "status", label: "Status" },
  { key: "size", label: "Size" },
  { key: "time", label: "Time" },
  { key: "waterfall", label: "Waterfall" },
] as const;

export const GRID_COLS_CLASS = "grid-cols-[24px_5fr_1fr_1fr_1fr_1fr_2fr]";

export const ROW_BASE_CLASSES =
  "grid gap-4 p-3 border-b border-border cursor-pointer hover:bg-muted/50 transition-colors";
export const SELECTED_ROW_CLASSES = "bg-accent/10 border-l-4 border-l-accent";
export const PINNED_ROW_CLASSES = "border-l-4 border-l-slate-600";
export const HEADER_CLASSES = "text-xs font-medium text-muted-foreground uppercase tracking-wide";
