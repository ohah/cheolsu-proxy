import { lazy, Suspense } from "react";
import {
  createBrowserRouter,
  RouterProvider as ReactRouterProvider,
  Outlet,
} from "react-router-dom";
import { AppSidebar } from "@/shared/app-sidebar";

const NetworkDashboard = lazy(() =>
  import("@/pages/network-dashboard").then((m) => ({
    default: m.NetworkDashboard,
  })),
);
const InterceptRulesPage = lazy(() =>
  import("@/pages/intercept-rules").then((m) => ({
    default: m.InterceptRulesPage,
  })),
);
const MapRulesPage = lazy(() =>
  import("@/pages/map-rules").then((m) => ({ default: m.MapRulesPage })),
);
const WebSocketDashboard = lazy(() =>
  import("@/pages/websocket-dashboard").then((m) => ({
    default: m.WebSocketDashboard,
  })),
);
const SseDashboard = lazy(() =>
  import("@/pages/sse-dashboard").then((m) => ({
    default: m.SseDashboard,
  })),
);
const GraphqlDashboard = lazy(() =>
  import("@/pages/graphql-dashboard").then((m) => ({
    default: m.GraphqlDashboard,
  })),
);
const SettingsPage = lazy(() =>
  import("@/pages/settings").then((m) => ({ default: m.SettingsPage })),
);
const ServerReplayPage = lazy(() =>
  import("@/pages/server-replay").then((m) => ({
    default: m.ServerReplayPage,
  })),
);
const ScriptPage = lazy(() => import("@/pages/script").then((m) => ({ default: m.ScriptPage })));
const BreakpointPage = lazy(() =>
  import("@/pages/breakpoint").then((m) => ({ default: m.BreakpointPage })),
);
const HostMappingPage = lazy(() =>
  import("@/pages/host-mapping").then((m) => ({
    default: m.HostMappingPage,
  })),
);
const ReverseProxyPage = lazy(() =>
  import("@/pages/reverse-proxy").then((m) => ({
    default: m.ReverseProxyPage,
  })),
);
const LogsPage = lazy(() => import("@/pages/log-viewer").then((m) => ({ default: m.LogsPage })));
const ContractTestingPage = lazy(() =>
  import("@/pages/contract-testing").then((m) => ({
    default: m.ContractTestingPage,
  })),
);

function RootLayout() {
  return (
    <div className="flex h-[100vh] w-full">
      <AppSidebar />
      <Suspense>
        <Outlet />
      </Suspense>
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
        path: "/sse",
        element: <SseDashboard />,
      },
      {
        path: "/graphql",
        element: <GraphqlDashboard />,
      },
      {
        path: "/server-replay",
        element: <ServerReplayPage />,
      },
      {
        path: "/script",
        element: <ScriptPage />,
      },
      {
        path: "/breakpoint",
        element: <BreakpointPage />,
      },
      {
        path: "/host-mapping",
        element: <HostMappingPage />,
      },
      {
        path: "/reverse-proxy",
        element: <ReverseProxyPage />,
      },
      {
        path: "/settings",
        element: <SettingsPage />,
      },
      {
        path: "/logs",
        element: <LogsPage />,
      },
      {
        path: "/contract-testing",
        element: <ContractTestingPage />,
      },
    ],
  },
]);

export const RouterProvider = () => {
  return <ReactRouterProvider router={router} />;
};
