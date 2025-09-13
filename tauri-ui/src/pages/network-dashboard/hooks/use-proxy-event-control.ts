import { useState, useEffect, useCallback } from 'react';

import { listen } from '@tauri-apps/api/event';

import type { ProxyEventTuple, HttpTransaction } from '@/entities/proxy';

interface UseProxyEventControlProps {
  onTransactionReceived: (transaction: HttpTransaction) => void;
  initialPaused?: boolean;
}

export const useProxyEventControl = ({
  onTransactionReceived,
  initialPaused = false
}: UseProxyEventControlProps) => {
  const [paused, setPaused] = useState<boolean>(initialPaused);

  const togglePause = useCallback(() => setPaused(prev => !prev), []);
  const pause = useCallback(() => setPaused(true), []);
  const resume = useCallback(() => setPaused(false), []);

  useEffect(() => {
    if (paused) return;

    const unlisten = listen<ProxyEventTuple>('proxy_event', (event) => {
      const [request, response] = event.payload;
      onTransactionReceived({ request, response });
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [paused]);

  return { paused, togglePause, pause, resume };
};
