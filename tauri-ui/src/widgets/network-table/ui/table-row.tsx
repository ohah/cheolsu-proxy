import { memo, useMemo } from 'react';

import { PathCell, MethodCell, StatusCell, SizeCell, TimeCell, ActionCell } from './cells';

import { ROW_BASE_CLASSES, SELECTED_ROW_CLASSES, GRID_COLS_CLASS } from '../model';
import type { TableRowData } from '../model';

interface TableRowProps {
  data: TableRowData;
  onSelect: () => void;
  onDelete: () => void;
}

export const TableRow = memo(function TableRow({ data, onSelect, onDelete }: TableRowProps) {
  const { isSelected } = data;

  const rowClasses = useMemo(() => {
    return `${ROW_BASE_CLASSES} ${GRID_COLS_CLASS} ${isSelected ? SELECTED_ROW_CLASSES : ''}`;
  }, [isSelected]);

  return (
    <div className={rowClasses} onClick={onSelect}>
      <PathCell data={data} />
      <MethodCell data={data} />
      <StatusCell data={data} />
      <SizeCell data={data} />
      <TimeCell data={data} />
      <ActionCell onDelete={onDelete} />
    </div>
  );
});

TableRow.displayName = 'TableRow';
