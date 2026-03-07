import {
  createBrowserRouter,
  RouterProvider as ReactRouterProvider,
  Outlet,
} from "react-router-dom";
import { NetworkDashboard } from "@/pages/network-dashboard";
import { InterceptRulesPage } from "@/pages/intercept-rules";
import { MapRulesPage } from "@/pages/map-rules";
import { WebSocketDashboard } from "@/pages/websocket-dashboard";
import { SettingsPage } from "@/pages/settings";
import { AppSidebar } from "@/shared/app-sidebar";

function RootLayout() {
  return (
    <div className="flex h-[100vh] w-full">
      <AppSidebar />
      <Outlet />
    </div>
  );
}

export const router = createBrowserRouter([
  {
    element: <RootLayout />,
    children: [
      {
        path: "/",
        element: <NetworkDashboard />,
      },
      {
        path: "/dashboard",
        element: <NetworkDashboard />,
      },
      {
        path: "/intercept-rules",
        element: <InterceptRulesPage />,
      },
      {
        path: "/map-rules",
        element: <MapRulesPage />,
      },
      {
        path: "/websocket",
        element: <WebSocketDashboard />,
      },
      {
        path: "/settings",
        element: <SettingsPage />,
      },
    ],
  },
]);

export const RouterProvider = () => {
  return <ReactRouterProvider router={router} />;
};
