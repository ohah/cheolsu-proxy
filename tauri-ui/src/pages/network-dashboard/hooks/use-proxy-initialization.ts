import { useEffect, useRef, useState } from 'react';

import { startProxyV2 } from '@/shared/api/proxy';
import { toast } from 'sonner';

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
        toast.success('Proxy started successfully');
      } catch (error) {
        console.error('Failed to start proxy:', error);
        isInitialized.current = false;
        setIsConnected(false);
        toast.error('Failed to start proxy');
      }
    };

    initializeProxy();
  }, [port]);

  return { isInitialized: isInitialized.current, isConnected };
};
