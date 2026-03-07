import { Network, Shield, Route, Plug, Settings } from "lucide-react";

import type { SidebarSection } from "./types";

export const DEFAULT_ACTIVE_SECTION = "network";

export const SIDEBAR_SECTIONS: SidebarSection[] = [
  {
    id: "network",
    label: "Network",
    icon: Network,
    description: "HTTP requests and responses",
  },
  {
    id: "websocket",
    label: "WebSocket",
    icon: Plug,
    description: "WebSocket connections and messages",
  },
  {
    id: "intercept-rules",
    label: "Intercept Rules",
    icon: Shield,
    description: "Manage request/response intercept rules",
  },
  {
    id: "map-rules",
    label: "Map Rules",
    icon: Route,
    description: "Map Local / Map Remote rules",
  },
  {
    id: "settings",
    label: "Settings",
    icon: Settings,
    description: "Proxy settings and configuration",
  },
];
