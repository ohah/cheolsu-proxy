import { useState, useMemo, useCallback } from 'react';
import type { HttpTransaction } from '@/entities/proxy';
import { parseFilterQuery } from '@/shared/lib/query-parser';
import { getFilteredTransactions } from '../lib';

interface UseTransactionFiltersProps {
  transactions: HttpTransaction[];
}

export const useTransactionFilters = ({ transactions }: UseTransactionFiltersProps) => {
  const [filterQueryString, setFilterQueryString] = useState<string>('');
  const [appliedQueryString, setAppliedQueryString] = useState<string>('');

  const parsedQuery = useMemo(() => parseFilterQuery(appliedQueryString), [appliedQueryString]);

  const filteredTransactions = useMemo(() => {
    return getFilteredTransactions(
      transactions,
      parsedQuery.status,
      parsedQuery.methods,
      parsedQuery.urls,
      parsedQuery.excludeStatus,
      parsedQuery.excludeMethods,
      parsedQuery.excludeUrls,
      parsedQuery.operator,
    );
  }, [transactions, parsedQuery]);

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
    totalCount: transactions.length,
    onFilterQueryChange: handleFilterQueryChange,
    onApplyFilter: handleApplyFilter,
  };
};
