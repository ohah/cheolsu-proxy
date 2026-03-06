import { TABLE_COLUMNS, GRID_COLS_CLASS, HEADER_CLASSES } from "../model";

interface TableHeaderProps {
  allChecked: boolean;
  someChecked: boolean;
  onToggleAll: () => void;
}

export const TableHeader = ({ allChecked, someChecked, onToggleAll }: TableHeaderProps) => {
  return (
    <div className="border-b border-border bg-background">
      <div className={`grid ${GRID_COLS_CLASS} gap-4 p-3.5 ${HEADER_CLASSES}`}>
        <div className="flex items-center justify-center w-5">
          <input
            type="checkbox"
            checked={allChecked}
            ref={(el) => {
              if (el) el.indeterminate = someChecked && !allChecked;
            }}
            onChange={onToggleAll}
            className="cursor-pointer accent-primary"
          />
        </div>
        {TABLE_COLUMNS.map((column) => (
          <div key={column.key} className={`col-span-${column.span}`}>
            {column.label}
          </div>
        ))}
      </div>
    </div>
  );
};
