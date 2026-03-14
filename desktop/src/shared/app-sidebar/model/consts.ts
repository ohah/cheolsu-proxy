import {
  Network,
  Shield,
  Route,
  Plug,
  Radio,
  Settings,
  Database,
  FileCode,
  PauseCircle,
  Globe,
  ScrollText,
} from "lucide-react";
import { msg } from "@lingui/core/macro";

import type { SidebarSection } from "./types";

export const DEFAULT_ACTIVE_SECTION = "network";

export const SIDEBAR_SECTIONS: SidebarSection[] = [
  {
    id: "network",
    path: "/dashboard",
    label: msg`Network`,
    icon: Network,
    description: msg`HTTP requests and responses`,
  },
  {
    id: "websocket",
    path: "/websocket",
    label: msg`WebSocket`,
    icon: Plug,
    description: msg`WebSocket connections and messages`,
  },
  {
    id: "sse",
    path: "/sse",
    label: msg`SSE`,
    icon: Radio,
    description: msg`Server-Sent Events streams`,
  },
  {
    id: "intercept-rules",
    path: "/intercept-rules",
    label: msg`Intercept Rules`,
    icon: Shield,
    description: msg`Manage request/response intercept rules`,
  },
  {
    id: "map-rules",
    path: "/map-rules",
    label: msg`Map Rules`,
    icon: Route,
    description: msg`Map Local / Map Remote rules`,
  },
  {
    id: "server-replay",
    path: "/server-replay",
    label: msg`Server Replay`,
    icon: Database,
    description: msg`Return cached responses for matching requests`,
  },
  {
    id: "breakpoint",
    path: "/breakpoint",
    label: msg`Breakpoints`,
    icon: PauseCircle,
    description: msg`Pause and inspect requests/responses`,
  },
  {
    id: "host-mapping",
    path: "/host-mapping",
    label: msg`Host Mapping`,
    icon: Globe,
    description: msg`Map DNS hostnames to different targets`,
  },
  {
    id: "script",
    path: "/script",
    label: msg`Script`,
    icon: FileCode,
    description: msg`Load and manage proxy scripts`,
  },
  {
    id: "settings",
    path: "/settings",
    label: msg`Settings`,
    icon: Settings,
    description: msg`Proxy settings and configuration`,
  },
  {
    id: "logs",
    path: "/logs",
    label: msg`Logs`,
    icon: ScrollText,
    description: msg`View application log files`,
  },
];
