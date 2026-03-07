import { createBrowserRouter, RouterProvider as ReactRouterProvider } from "react-router-dom";
import { NetworkDashboard } from "@/pages/network-dashboard";
import { InterceptRulesPage } from "@/pages/intercept-rules";
import { MapRulesPage } from "@/pages/map-rules";
import { WebSocketDashboard } from "@/pages/websocket-dashboard";
import { SettingsPage } from "@/pages/settings";

// React Router v7의 기본 라우터 설정
export const router = createBrowserRouter([
  {
    path: "/",
    element: <NetworkDashboard />,
    handle: {
      title: "Cheolsu Proxy - Network Dashboard",
      description: "HTTP proxy monitoring and debugging tool",
    },
  },
  {
    path: "/dashboard",
    element: <NetworkDashboard />,
    handle: {
      title: "Dashboard - Cheolsu Proxy",
      description: "Network traffic monitoring dashboard",
    },
  },
  {
    path: "/intercept-rules",
    element: <InterceptRulesPage />,
    handle: {
      title: "Intercept Rules - Cheolsu Proxy",
      description: "Manage request/response intercept rules",
    },
  },
  {
    path: "/map-rules",
    element: <MapRulesPage />,
    handle: {
      title: "Map Rules - Cheolsu Proxy",
      description: "Map Local / Map Remote rules",
    },
  },
  {
    path: "/websocket",
    element: <WebSocketDashboard />,
    handle: {
      title: "WebSocket - Cheolsu Proxy",
      description: "WebSocket connections and messages",
    },
  },
  {
    path: "/settings",
    element: <SettingsPage />,
    handle: {
      title: "Settings - Cheolsu Proxy",
      description: "Application settings and configuration",
    },
  },
]);

export const RouterProvider = () => {
  return <ReactRouterProvider router={router} />;
};
