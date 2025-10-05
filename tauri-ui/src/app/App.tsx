import { useEffect } from 'react';
import { useThemeProvider, RouterProvider } from './providers';
import { Toaster } from '@/shared/ui';
import { useProxyStore } from '@/shared/stores';

const App: React.FC = () => {
  useThemeProvider();
  const { initializeProxy } = useProxyStore();

  // 앱 시작 시 프록시 초기화
  useEffect(() => {
    initializeProxy();
  }, [initializeProxy]);

  return (
    <div className="App">
      <RouterProvider />
      <Toaster richColors />
    </div>
  );
};

export default App;
