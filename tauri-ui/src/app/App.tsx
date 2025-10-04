import { useThemeProvider, RouterProvider } from './providers';
import { Toaster } from '@/shared/ui';

const App: React.FC = () => {
  useThemeProvider();

  return (
    <div className="App">
      <RouterProvider />
      <Toaster richColors />
    </div>
  );
};

export default App;
