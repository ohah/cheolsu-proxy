import { useEffect } from 'react';

import { listen } from '@tauri-apps/api/event';

import type { ProxyEventTuple, HttpTransaction } from '@/entities/proxy';
import { useTransactionStore } from '@/shared/stores';

/**
 * 전역 Proxy 이벤트 리스너 Provider
 * 앱 전체에서 proxy_event를 수신하여 transaction store에 저장합니다.
 * 탭 이동과 무관하게 이벤트를 계속 수신할 수 있습니다.
 */
export const ProxyEventProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { addTransaction, isPaused } = useTransactionStore();

  useEffect(() => {
    const unlisten = listen<ProxyEventTuple>('proxy_event', (event) => {
      // isPaused 상태와 관계없이 이벤트는 수신하되, store에 추가할지만 결정
      if (isPaused) return;

      const [request, response] = event.payload;
      const transaction: HttpTransaction = { request, response };

      // zustand store에 transaction 추가
      addTransaction(transaction);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [addTransaction, isPaused]);

  return <>{children}</>;
};
