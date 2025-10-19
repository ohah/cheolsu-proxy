import { HelpCircle } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/shared/ui';

export const FilterHelpTooltip = () => {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button className="p-2 hover:bg-accent/10 rounded-md transition-colors" aria-label="Query syntax help">
          <HelpCircle className="w-4 h-4 text-muted-foreground" />
        </button>
      </TooltipTrigger>
      <TooltipContent
        side="bottom"
        align="start"
        className="w-96 p-4 bg-popover text-popover-foreground border border-border"
      >
        <div className="space-y-3">
          <p className="font-semibold text-sm text-foreground">쿼리 문법 가이드</p>

          <div className="space-y-2 text-xs">
            <div className="font-mono bg-muted py-2 rounded">
              <span className="text-foreground font-bold">method</span>
              <span className="text-muted-foreground">=</span>
              <span className="text-accent font-semibold">"GET,POST"</span>
            </div>
            <p className="text-muted-foreground">GET 또는 POST 메서드</p>

            <div className="font-mono bg-muted py-2 rounded">
              <span className="text-foreground font-bold">status</span>
              <span className="text-muted-foreground">=</span>
              <span className="text-accent font-semibold">"2xx,404"</span>
            </div>
            <p className="text-muted-foreground">2xx 성공 또는 404 에러</p>

            <div className="font-mono bg-muted py-2 rounded">
              <span className="text-foreground font-bold">url</span>
              <span className="text-muted-foreground">|=</span>
              <span className="text-accent font-semibold">"api"</span>
            </div>
            <p className="text-muted-foreground">URL에 "api" 포함</p>

            <div className="font-mono bg-muted py-2 rounded">
              <span className="text-foreground font-bold">method</span>
              <span className="text-destructive">!=</span>
              <span className="text-accent font-semibold">"OPTIONS"</span>
            </div>
            <p className="text-muted-foreground">OPTIONS 메서드 제외</p>
          </div>

          <div className="pt-2 border-t border-border">
            <p className="text-muted-foreground text-xs">
              여러 조건: <code className="bg-muted px-1 py-0.5 rounded text-foreground">method="GET" status="2xx"</code>
            </p>
            <p className="text-muted-foreground text-xs mt-1">
              논리 OR:{' '}
              <code className="bg-muted px-1 py-0.5 rounded text-foreground">method="GET" or status="5xx"</code>
            </p>
            <p className="text-muted-foreground text-xs mt-2 flex items-center gap-1">
              적용:
              <kbd className="px-1.5 py-0.5 bg-muted rounded text-[10px] border border-border text-foreground">⌘</kbd>
              <span>+</span>
              <kbd className="px-1.5 py-0.5 bg-muted rounded text-[10px] border border-border text-foreground">
                Enter
              </kbd>
            </p>
          </div>
        </div>
      </TooltipContent>
    </Tooltip>
  );
};
