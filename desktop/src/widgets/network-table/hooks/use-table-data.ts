import { useMemo } from "react";

import type { HttpTransaction } from "@/entities/proxy";

import { getAuthority } from "../lib";
import type { TableRowData } from "../model";

/** 트랜잭션 집합에 대한 Waterfall 시간축 범위를 계산한다. */
export function computeTimelineRange(transactions: HttpTransaction[]): {
  start: number;
  end: number;
} {
  let minStart = Infinity;
  let maxEnd = -Infinity;
  for (const transaction of transactions) {
    const reqTime = transaction.request?.time;
    const resTime = transaction.response?.time;
    if (reqTime != null && reqTime < minStart) minStart = reqTime;
    if (resTime != null && resTime > maxEnd) maxEnd = resTime;
    // 응답이 없으면 요청 시각을 끝으로 간주
    if (reqTime != null && resTime == null && reqTime > maxEnd) maxEnd = reqTime;
  }
  if (!isFinite(minStart)) return { start: 0, end: 0 };
  // 최소 범위 100ms (너무 짧으면 바가 안 보임)
  if (maxEnd - minStart < 100_000_000) maxEnd = minStart + 100_000_000;
  return { start: minStart, end: maxEnd };
}

interface UseTableDataProps {
  transactions: HttpTransaction[];
  selectedTransaction: HttpTransaction | null;
  /** 공유 Waterfall 시간축. 지정하지 않으면 transactions로 자체 계산한다. */
  timelineRange?: { start: number; end: number };
}

export const useTableData = ({
  transactions,
  selectedTransaction,
  timelineRange,
}: UseTableDataProps) => {
  const selectedTime = useMemo(
    () => selectedTransaction?.request?.time,
    [selectedTransaction?.request?.time],
  );

  const processedTransactions = useMemo(() => {
    return transactions.map((transaction, index) => {
      const { request, response } = transaction;

      const timeDiff =
        response?.time && request?.time ? Math.trunc((response.time - request.time) / 1e6) : "N/A";

      let authority = "";
      let pathname = "";

      if (request?.uri) {
        // CONNECT 요청의 경우 host:port 형식이므로 특별 처리
        if (request.method === "CONNECT") {
          authority = request.uri; // host:port 형식 그대로 사용
          pathname = "/"; // CONNECT 요청은 경로가 없으므로 '/'로 표시
        } else {
          try {
            const url = new URL(request.uri);
            authority = getAuthority(request.uri);
            pathname = url.pathname;
          } catch {
            authority = request.uri.split("/")[0] || request.uri;
            pathname = "";
          }
        }
      }

      return {
        transaction,
        index,
        timeDiff,
        authority,
        pathname,
        requestTime: request?.time,
      };
    });
  }, [transactions]);

  // Waterfall 시간축 범위. 공유 범위가 주어지면 그것을 사용(pinned/unpinned가 동일 축을 쓰도록),
  // 없으면 자체 계산한다.
  const { timelineStart, timelineEnd } = useMemo(() => {
    const range = timelineRange ?? computeTimelineRange(transactions);
    return { timelineStart: range.start, timelineEnd: range.end };
  }, [transactions, timelineRange]);

  const tableData = useMemo<TableRowData[]>(() => {
    return processedTransactions.map((item) => ({
      ...item,
      isSelected: selectedTime === item.requestTime,
      timelineStart,
      timelineEnd,
    }));
  }, [processedTransactions, selectedTime, timelineStart, timelineEnd]);

  return { tableData };
};
