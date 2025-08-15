
import React, { useState, useEffect } from 'react';
import { fetchProxyStatus } from '../api';
import ProxyOff from './ProxyOff';
import ProxyOn from './ProxyOn';
import { useThemeProvider } from '../hooks/use-theme-provider';

const App: React.FC = () => {
  const [isProxyOn, setIsProxyOn] = useState(false);

  useEffect(() => {
    const getStatus = async () => {
      try {
        const status = await fetchProxyStatus();
        setIsProxyOn(status);
      } catch (err) {
        console.error('Failed to fetch proxy status:', err);
      }
    };
    getStatus();
  }, []);

  const handleStart = () => {
    setIsProxyOn(true);
  };

  const handleStop = () => {
    setIsProxyOn(false);
  };

  useThemeProvider();

  return (
    <main>
      {/* <TitleBar /> */}
      {isProxyOn ? (
        <ProxyOn onStop={handleStop} />
      ) : (
        <ProxyOff onStart={handleStart} />
      )}
    </main>
  );
};

export default App;
