import { useEffect, useRef, useState } from 'react';

import { startProxyV2 } from '@/shared/api/proxy';

export const useProxyInitialization = (port: number = 8100) => {
  const isInitialized = useRef(false);

  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    if (isInitialized.current) return;

    const initializeProxy = async () => {
      try {
        const response = await startProxyV2(port);
        console.log('response: ', response);
        setIsConnected(true);
        isInitialized.current = true;
      } catch (error) {
        console.error('Failed to start proxy:', error);
        setIsConnected(false);
      }
    };

    initializeProxy();
  }, [port]);

  return { isInitialized: isInitialized.current, isConnected };
};
