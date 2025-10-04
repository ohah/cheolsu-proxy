import { memo } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Button } from '@/shared/ui';
import { SIDEBAR_SECTIONS } from '../model';
import type { SidebarSection } from '../model';

interface SidebarNavigationProps {
  collapsed: boolean;
}

export const SidebarNavigation = memo(({ collapsed }: SidebarNavigationProps) => {
  const navigate = useNavigate();
  const location = useLocation();

  // 현재 경로를 기반으로 활성 섹션 결정
  const getActiveSection = () => {
    switch (location.pathname) {
      case '/':
      case '/dashboard':
        return 'network';
      case '/sessions':
        return 'sessions';
      default:
        return 'network';
    }
  };

  const activeSection = getActiveSection();

  const handleSectionClick = (section: SidebarSection) => {
    // 라우터 네비게이션으로만 처리
    switch (section.id) {
      case 'network':
        navigate('/dashboard');
        break;
      case 'sessions':
        navigate('/sessions');
        break;
      // 향후 다른 섹션들에 대한 라우팅 추가
      // case 'performance':
      //   navigate('/performance');
      //   break;
      default:
        navigate('/dashboard');
    }
  };

  return (
    <div className="space-y-1">
      {SIDEBAR_SECTIONS.map((section: SidebarSection) => {
        const Icon = section.icon;
        const isActive = activeSection === section.id;

        return (
          <Button
            key={section.id}
            variant="ghost"
            className={`w-full justify-start gap-3 ${
              isActive
                ? 'bg-accent text-accent-foreground dark:hover:bg-accent/50'
                : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
            }`}
            onClick={() => handleSectionClick(section)}
            title={section.description}
          >
            <Icon className="w-4 h-4" />
            {collapsed ? null : <span className="flex-1 text-left">{section.label}</span>}
          </Button>
        );
      })}
    </div>
  );
});

SidebarNavigation.displayName = 'SidebarNavigation';
