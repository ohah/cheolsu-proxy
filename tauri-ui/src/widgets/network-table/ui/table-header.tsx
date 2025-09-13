import { TABLE_COLUMNS, GRID_COLS_CLASS, HEADER_CLASSES } from '../model';

export const TableHeader = () => {
  return (
    <div className="border-b border-border bg-muted/50">
      <div className={`grid ${GRID_COLS_CLASS} gap-4 p-3 ${HEADER_CLASSES}`}>
        {TABLE_COLUMNS.map((column) => (
          <div key={column.key} className={`col-span-${column.span}`}>
            {column.label}
          </div>
        ))}
      </div>
    </div>
  );
};
