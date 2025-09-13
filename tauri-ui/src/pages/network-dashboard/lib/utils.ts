import type { HttpTransaction } from '@/entities/proxy';

const getStatusRange = (status: number): number => {
  return Math.floor(status / 100) * 100;
};

const matchesStatusFilter = (status: number, statusRanges: number[]): boolean => {
  if (statusRanges.length === 0) return true;
  return statusRanges.includes(getStatusRange(status));
};

const matchesMethodFilter = (method: string, methodFilter: string[]): boolean => {
  if (methodFilter.length === 0) return true;
  return methodFilter.includes(method);
};

const matchesPathFilter = (path: string, pathFilter: string): boolean => {
  if (pathFilter.length === 0) return true;
  return path.includes(pathFilter);
};

export const getFilteredTransactions = (
  transactions: HttpTransaction[],
  statusFilter: string[],
  methodFilter: string[],
  pathFilter: string
): HttpTransaction[] => {
  if (statusFilter.length === 0 && methodFilter.length === 0 && pathFilter.length === 0) {
    return transactions;
  }

  const statusRanges = statusFilter.map(Number);

  return transactions.filter(transaction => {
    const status = transaction.response?.status ?? 0;
    const method = transaction.request?.method ?? "";
    const path = transaction.request?.uri ?? "";

    return matchesStatusFilter(status, statusRanges) &&
           matchesMethodFilter(method, methodFilter) &&
           matchesPathFilter(path, pathFilter);
  });
};
