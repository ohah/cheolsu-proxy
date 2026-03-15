import { useState, useMemo, useCallback } from "react";
import type { HttpTransaction } from "@/entities/proxy";
import { parseFilterQuery } from "@/shared/lib/query-parser";
import { getFilteredTransactions } from "../lib";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";

interface UseTransactionFiltersProps {
  transactions: HttpTransaction[];
}

export const useTransactionFilters = ({ transactions }: UseTransactionFiltersProps) => {
  const [filterQueryString, setFilterQueryString] = useState<string>("");
  const [appliedQueryString, setAppliedQueryString] = useState<string>("");
  const showConnectRequests = useAppSettingsStore((s) => s.showConnectRequests);
  const clientTags = useAppSettingsStore((s) => s.clientTags);

  const parsedQuery = useMemo(() => parseFilterQuery(appliedQueryString), [appliedQueryString]);

  const visibleTransactions = useMemo(() => {
    if (showConnectRequests) return transactions;
    return transactions.filter((t) => t.request?.method !== "CONNECT");
  }, [transactions, showConnectRequests]);

  const filteredTransactions = useMemo(() => {
    return getFilteredTransactions(
      visibleTransactions,
      parsedQuery.status,
      parsedQuery.methods,
      parsedQuery.urls,
      parsedQuery.excludeStatus,
      parsedQuery.excludeMethods,
      parsedQuery.excludeUrls,
      parsedQuery.clients,
      parsedQuery.excludeClients,
      clientTags,
      parsedQuery.operator,
    );
  }, [visibleTransactions, parsedQuery, clientTags]);

  const handleFilterQueryChange = useCallback((query: string) => {
    setFilterQueryString(query);
  }, []);

  const handleApplyFilter = useCallback((query: string) => {
    setAppliedQueryString(query);
    setFilterQueryString(query);
  }, []);

  return {
    filterQueryString,
    appliedQueryString,
    filteredTransactions,
    filteredCount: filteredTransactions.length,
    totalCount: visibleTransactions.length,
    onFilterQueryChange: handleFilterQueryChange,
    onApplyFilter: handleApplyFilter,
  };
};
