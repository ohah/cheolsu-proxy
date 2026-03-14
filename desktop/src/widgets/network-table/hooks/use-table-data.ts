import { useMemo } from "react";

import type { HttpTransaction } from "@/entities/proxy";

import { getAuthority } from "../lib";
import type { TableRowData } from "../model";

interface UseTableDataProps {
  transactions: HttpTransaction[];
  selectedTransaction: HttpTransaction | null;
}

export const useTableData = ({ transactions, selectedTransaction }: UseTableDataProps) => {
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

  // Waterfall 시간축 범위 계산
  const { timelineStart, timelineEnd } = useMemo(() => {
    let minStart = Infinity;
    let maxEnd = -Infinity;

    for (const { transaction } of processedTransactions) {
      const reqTime = transaction.request?.time;
      const resTime = transaction.response?.time;
      if (reqTime != null && reqTime < minStart) minStart = reqTime;
      if (resTime != null && resTime > maxEnd) maxEnd = resTime;
      // 응답이 없으면 요청 시각을 끝으로 간주
      if (reqTime != null && resTime == null && reqTime > maxEnd) maxEnd = reqTime;
    }

    if (!isFinite(minStart)) return { timelineStart: 0, timelineEnd: 0 };
    // 최소 범위 100ms (너무 짧으면 바가 안 보임)
    if (maxEnd - minStart < 100_000_000) maxEnd = minStart + 100_000_000;
    return { timelineStart: minStart, timelineEnd: maxEnd };
  }, [processedTransactions]);

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
