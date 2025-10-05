import { createBrowserRouter, RouterProvider as ReactRouterProvider } from 'react-router-dom';
import { NetworkDashboard } from '@/pages/network-dashboard';
import { SessionsPage } from '@/pages/sessions';

// React Router v7의 기본 라우터 설정
export const router = createBrowserRouter([
  {
    path: '/',
    element: <NetworkDashboard />,
    handle: {
      title: 'Cheolsu Proxy - Network Dashboard',
      description: 'HTTP proxy monitoring and debugging tool',
    },
  },
  {
    path: '/dashboard',
    element: <NetworkDashboard />,
    handle: {
      title: 'Dashboard - Cheolsu Proxy',
      description: 'Network traffic monitoring dashboard',
    },
  },
  {
    path: '/sessions',
    element: <SessionsPage />,
    handle: {
      title: 'Sessions - Cheolsu Proxy',
      description: 'View and manage saved HTTP sessions',
    },
  },
  // 향후 추가될 페이지들을 위한 라우트들
  // {
  //   path: '/settings',
  //   element: <SettingsPage />,
  //   handle: {
  //     title: 'Settings - Cheolsu Proxy',
  //     description: 'Application settings and configuration',
  //   },
  // },
  // {
  //   path: '/about',
  //   element: <AboutPage />,
  //   handle: {
  //     title: 'About - Cheolsu Proxy',
  //     description: 'About Cheolsu Proxy application',
  //   },
  // },
]);

export const RouterProvider = () => {
  return <ReactRouterProvider router={router} />;
};
