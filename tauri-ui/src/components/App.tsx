import React, { useState, useEffect } from 'react';
import { fetchProxyStatus } from '../api';
import ProxyOff from './ProxyOff';
import ProxyOn from './ProxyOn';
import { ProxyV2Control } from './ProxyV2Control';
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
    <main className="min-h-screen bg-gray-50 dark:bg-gray-900">
      {/* <TitleBar /> */}
      <div className="container mx-auto py-8 px-4">
        <h1 className="text-4xl font-bold text-center mb-8 text-gray-900 dark:text-white">Cheolsu Proxy</h1>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          <div>{isProxyOn ? <ProxyOn onStop={handleStop} /> : <ProxyOff onStart={handleStart} />}</div>

          <div>
            <ProxyV2Control />
          </div>
        </div>
      </div>
    </main>
  );
};

export default App;
