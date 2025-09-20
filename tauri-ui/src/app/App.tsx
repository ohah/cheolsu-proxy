import { NetworkDashboard } from '@/pages/network-dashboard';

import { useThemeProvider } from './providers';
import { Toaster } from '@/shared/ui';

const App: React.FC = () => {
  useThemeProvider();

  return (
    <div className="App">
      <NetworkDashboard />
      <Toaster richColors />
    </div>
  );
};

export default App;
