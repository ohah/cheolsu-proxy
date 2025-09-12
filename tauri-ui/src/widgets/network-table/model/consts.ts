export const TABLE_COLUMNS = [
  { key: 'path', label: 'Path', span: 5 },
  { key: 'method', label: 'Method', span: 1 },
  { key: 'status', label: 'Status', span: 1 },
  { key: 'size', label: 'Size', span: 1 },
  { key: 'time', label: 'Time', span: 1 },
  { key: 'action', label: 'Action', span: 1 },
] as const;

export const GRID_COLS_CLASS = 'grid-cols-10';

export const ROW_BASE_CLASSES = 'grid gap-4 p-3 border-b border-border cursor-pointer hover:bg-muted/50 transition-colors';
export const SELECTED_ROW_CLASSES = 'bg-accent/10 border-l-4 border-l-accent';
export const HEADER_CLASSES = 'text-xs font-medium text-muted-foreground uppercase tracking-wide';
