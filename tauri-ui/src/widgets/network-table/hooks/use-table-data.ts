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

  const tableData = useMemo<TableRowData[]>(() => {
    return processedTransactions.map((item) => ({
      ...item,
      isSelected: selectedTime === item.requestTime,
    }));
  }, [processedTransactions, selectedTime]);

  return { tableData };
};
