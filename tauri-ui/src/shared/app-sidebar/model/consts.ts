import { Network, Shield, Route, Plug, Settings } from "lucide-react";
import { msg } from "@lingui/core/macro";

import type { SidebarSection } from "./types";

export const DEFAULT_ACTIVE_SECTION = "network";

export const SIDEBAR_SECTIONS: SidebarSection[] = [
  {
    id: "network",
    label: msg`Network`,
    icon: Network,
    description: msg`HTTP requests and responses`,
  },
  {
    id: "websocket",
    label: msg`WebSocket`,
    icon: Plug,
    description: msg`WebSocket connections and messages`,
  },
  {
    id: "intercept-rules",
    label: msg`Intercept Rules`,
    icon: Shield,
    description: msg`Manage request/response intercept rules`,
  },
  {
    id: "map-rules",
    label: msg`Map Rules`,
    icon: Route,
    description: msg`Map Local / Map Remote rules`,
  },
  {
    id: "settings",
    label: msg`Settings`,
    icon: Settings,
    description: msg`Proxy settings and configuration`,
  },
];
