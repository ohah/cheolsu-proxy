import { useEffect, useCallback } from 'react';

import { listen } from '@tauri-apps/api/event';

import type { ProxyEventTuple, HttpTransaction } from '@/entities/proxy';
import { useTransactionStore } from '@/shared/stores';

interface UseProxyEventControlProps {
  onTransactionReceived?: (transaction: HttpTransaction) => void;
  initialPaused?: boolean;
}

export const useProxyEventControl = ({
  onTransactionReceived,
  initialPaused = false,
}: UseProxyEventControlProps = {}) => {
  const { isPaused, addTransaction, setPaused, togglePause } = useTransactionStore();

  // 초기 paused 상태 설정
  useEffect(() => {
    if (initialPaused !== undefined) {
      setPaused(initialPaused);
    }
  }, [initialPaused, setPaused]);

  const pause = useCallback(() => setPaused(true), [setPaused]);
  const resume = useCallback(() => setPaused(false), [setPaused]);

  useEffect(() => {
    if (isPaused) return;

    const unlisten = listen<ProxyEventTuple>('proxy_event', (event) => {
      const [request, response] = event.payload;
      const transaction: HttpTransaction = { request, response };

      // zustand store에 transaction 추가
      addTransaction(transaction);

      // 기존 콜백이 있으면 호출 (하위 호환성)
      onTransactionReceived?.(transaction);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [isPaused, addTransaction, onTransactionReceived]);

  return {
    paused: isPaused,
    togglePause,
    pause,
    resume,
  };
};
