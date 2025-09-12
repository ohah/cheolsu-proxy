import { Network } from 'lucide-react';

import type { SidebarSection } from './types';

export const DEFAULT_ACTIVE_SECTION = 'network';

export const SIDEBAR_SECTIONS: SidebarSection[] = [
  {
    id: 'network',
    label: 'Network',
    icon: Network,
    description: 'HTTP requests and responses'
  },
  // {
  //   id: 'performance',
  //   label: 'Performance',
  //   icon: Activity,
  //   description: 'Request timing and performance metrics'
  // },
  // {
  //   id: 'errors',
  //   label: 'Errors',
  //   icon: AlertTriangle,
  //   description: 'Failed requests and errors'
  // },
  // {
  //   id: 'security',
  //   label: 'Security',
  //   icon: CheckCircle,
  //   description: 'Security issues and warnings'
  // },
  // {
  //   id: 'timing',
  //   label: 'Timing',
  //   icon: Clock,
  //   description: 'Request and response timing'
  // },
  // {
  //   id: 'analytics',
  //   label: 'Analytics',
  //   icon: BarChart3,
  //   description: 'Request analytics and insights'
  // },
];
