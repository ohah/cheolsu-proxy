import type { HttpTransaction } from "@/entities/proxy";

export interface TableRowData {
  transaction: HttpTransaction;
  index: number;
  timeDiff: string | number;
  authority: string;
  pathname: string;
  isSelected: boolean;
  requestTime?: number;
  /** Waterfall 시간축: 전체 트랜잭션 중 가장 이른 요청 시각 (나노초) */
  timelineStart?: number;
  /** Waterfall 시간축: 전체 트랜잭션 중 가장 늦은 종료 시각 (나노초) */
  timelineEnd?: number;
}

export interface TableCellProps {
  data: TableRowData;
}
