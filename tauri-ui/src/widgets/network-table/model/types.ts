import type { HttpTransaction } from '@/entities/proxy';

export interface TableRowData {
  transaction: HttpTransaction;
  index: number;
  timeDiff: string | number;
  authority: string;
  pathname: string;
  isSelected: boolean;
  requestTime?: number;
}

export interface TableCellProps {
  data: TableRowData;
}
