import { NetworkDashboard } from '@/pages/network-dashboard';

import { useThemeProvider } from './providers';

const App: React.FC = () => {
  useThemeProvider();

  return <NetworkDashboard />;
};

export default App;
