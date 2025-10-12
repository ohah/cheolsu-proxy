import { useCallback, useEffect } from 'react';

import type { HttpTransaction } from '@/entities/proxy';
import { useTransactionStore } from '@/shared/stores';

interface UseTransactionsProps {
  initialPaused?: boolean;
}

/**
 * Transaction 관련 모든 기능을 관리하는 통합 Hook
 * - Transaction 데이터 관리
 * - Transaction 선택 관리
 * - Proxy 이벤트 제어 (pause/resume)
 */
export const useTransactions = ({ initialPaused = false }: UseTransactionsProps = {}) => {
  const {
    transactions,
    selectedTransaction,
    isPaused,
    addTransaction,
    clearTransactions,
    deleteTransaction,
    setSelectedTransaction,
    setPaused,
    togglePause,
  } = useTransactionStore();

  // 초기 paused 상태 설정
  useEffect(() => {
    if (initialPaused !== undefined) {
      setPaused(initialPaused);
    }
  }, [initialPaused, setPaused]);

  const createTransactionSelectHandler = useCallback(
    (transaction: HttpTransaction) => () => {
      setSelectedTransaction(transaction);
    },
    [setSelectedTransaction],
  );

  const clearSelectedTransaction = useCallback(() => {
    setSelectedTransaction(null);
  }, [setSelectedTransaction]);

  const pause = useCallback(() => setPaused(true), [setPaused]);
  const resume = useCallback(() => setPaused(false), [setPaused]);

  return {
    // Transaction 데이터
    transactions,
    selectedTransaction,
    addTransaction,
    clearTransactions,
    deleteTransaction,

    // Transaction 선택 관리
    createTransactionSelectHandler,
    clearSelectedTransaction,

    // Proxy 이벤트 제어
    isPaused,
    paused: isPaused, // 하위 호환성을 위해 별칭 제공
    setPaused,
    togglePause,
    pause,
    resume,
  };
};
