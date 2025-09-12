import { useState, useMemo, useCallback } from 'react';

import type { HttpTransaction } from '@/entities/proxy';

import { getFilteredTransactions } from '../lib';

interface UseTransactionFiltersProps {
  transactions: HttpTransaction[];
}

export const useTransactionFilters = ({ transactions }: UseTransactionFiltersProps) => {
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [methodFilter, setMethodFilter] = useState<string[]>([]);
  const [statusFilter, setStatusFilter] = useState<string[]>([]);

  const filteredTransactions = useMemo(() => {
    return getFilteredTransactions(transactions, statusFilter, methodFilter, searchQuery);
  }, [transactions, statusFilter, methodFilter, searchQuery]);

  const filteredCount = useMemo(() => filteredTransactions.length, [filteredTransactions]);
  const totalCount = useMemo(() => transactions.length, [transactions]);

  const handleSearchQueryChange = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    setSearchQuery(event.target.value);
  }, []);

  return {
    searchQuery,
    filteredTransactions,
    filteredCount,
    totalCount,

    setMethodFilter,
    setStatusFilter,

    onSearchQueryChange: handleSearchQueryChange,
  };
};
