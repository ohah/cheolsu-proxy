import { useEffect, useRef, useState } from 'react';

import { startProxyV2 } from '@/shared/api/proxy';

export const useProxyInitialization = (port: number = 8100) => {
  const isInitialized = useRef(false);

  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    if (isInitialized.current) return;

    isInitialized.current = true;

    const initializeProxy = async () => {
      try {
        await startProxyV2(port);
        setIsConnected(true);
      } catch (error) {
        console.error('Failed to start proxy:', error);
        isInitialized.current = false;
        setIsConnected(false);
      }
    };

    initializeProxy();
  }, [port]);

  return { isInitialized: isInitialized.current, isConnected };
};
