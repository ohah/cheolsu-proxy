'use client';

import { useState } from 'react';

import { Network, Activity, Clock, AlertTriangle, CheckCircle, Settings, Filter, BarChart3 } from 'lucide-react';

import { Button, Badge } from '@/shared/ui';

export function NetworkSidebar() {
  const [activeSection, setActiveSection] = useState('network');

  const sections = [
    { id: 'network', label: 'Network', icon: Network, count: 5 },
    { id: 'performance', label: 'Performance', icon: Activity, count: 12 },
    { id: 'errors', label: 'Errors', icon: AlertTriangle, count: 2 },
    { id: 'security', label: 'Security', icon: CheckCircle, count: 0 },
    { id: 'timing', label: 'Timing', icon: Clock, count: 8 },
    { id: 'analytics', label: 'Analytics', icon: BarChart3, count: 15 },
  ];

  return (
    <div className="w-64 bg-sidebar border-r border-sidebar-border flex flex-col shrink-0">
      <div className="p-4 border-b border-sidebar-border">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 bg-sidebar-primary rounded-lg flex items-center justify-center">
            <Network className="w-4 h-4 text-sidebar-primary-foreground" />
          </div>
          <div>
            <h1 className="font-semibold text-sidebar-foreground">DevTools</h1>
            <p className="text-xs text-muted-foreground">Network Monitor</p>
          </div>
        </div>
      </div>

      <div className="flex-1 p-2">
        <div className="space-y-1">
          {sections.map((section) => {
            const Icon = section.icon;
            const isActive = activeSection === section.id;

            return (
              <Button
                key={section.id}
                variant={isActive ? 'secondary' : 'ghost'}
                className={`w-full justify-start gap-3 ${
                  isActive
                    ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                    : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
                }`}
                onClick={() => setActiveSection(section.id)}
              >
                <Icon className="w-4 h-4" />
                <span className="flex-1 text-left">{section.label}</span>
                {section.count > 0 && (
                  <Badge variant={isActive ? 'default' : 'secondary'} className="text-xs">
                    {section.count}
                  </Badge>
                )}
              </Button>
            );
          })}
        </div>

        <div className="mt-6 pt-4 border-t border-sidebar-border">
          <Button
            variant="ghost"
            className="w-full justify-start gap-3 text-sidebar-foreground hover:bg-sidebar-accent/50"
          >
            <Filter className="w-4 h-4" />
            <span>Filters</span>
          </Button>

          <Button
            variant="ghost"
            className="w-full justify-start gap-3 text-sidebar-foreground hover:bg-sidebar-accent/50"
          >
            <Settings className="w-4 h-4" />
            <span>Settings</span>
          </Button>
        </div>
      </div>

      <div className="p-4 border-t border-sidebar-border">
        <div className="text-xs text-muted-foreground">
          <div className="flex justify-between mb-1">
            <span>Status</span>
            <span className="text-green-600">Connected</span>
          </div>
          <div className="flex justify-between">
            <span>Version</span>
            <span>v1.2.1</span>
          </div>
        </div>
      </div>
    </div>
  );
}
