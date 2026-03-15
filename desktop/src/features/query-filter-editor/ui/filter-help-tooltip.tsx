import { HelpCircle } from "lucide-react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/shared/ui";

export const FilterHelpTooltip = () => {
  const { t } = useLingui();

  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <button
            className="p-2 hover:bg-accent/10 rounded-md transition-colors"
            aria-label={t`Query syntax help`}
          />
        }
      >
        <HelpCircle className="w-4 h-4 text-muted-foreground" />
      </TooltipTrigger>
      <TooltipContent
        side="bottom"
        align="start"
        className="w-96 p-4 bg-popover text-popover-foreground border border-border"
      >
        <div className="space-y-3">
          <p className="font-semibold text-sm text-foreground">
            <Trans>Query Syntax Guide</Trans>
          </p>

          <div className="space-y-2 text-xs">
            <div className="font-mono bg-muted py-2 rounded">
              <span className="text-foreground font-bold">method</span>
              <span className="text-muted-foreground">=</span>
              <span className="text-accent font-semibold">"GET,POST"</span>
            </div>
            <p className="text-muted-foreground">
              <Trans>Match GET or POST methods</Trans>
            </p>

            <div className="font-mono bg-muted py-2 rounded">
              <span className="text-foreground font-bold">status</span>
              <span className="text-muted-foreground">=</span>
              <span className="text-accent font-semibold">"2xx,404"</span>
            </div>
            <p className="text-muted-foreground">
              <Trans>Match 2xx success or 404 error</Trans>
            </p>

            <div className="font-mono bg-muted py-2 rounded">
              <span className="text-foreground font-bold">url</span>
              <span className="text-muted-foreground">|=</span>
              <span className="text-accent font-semibold">"api"</span>
            </div>
            <p className="text-muted-foreground">
              <Trans>URL contains "api"</Trans>
            </p>

            <div className="font-mono bg-muted py-2 rounded">
              <span className="text-foreground font-bold">client</span>
              <span className="text-muted-foreground">|=</span>
              <span className="text-accent font-semibold">"192.168"</span>
            </div>
            <p className="text-muted-foreground">
              <Trans>Filter by client IP, tag, or auth user</Trans>
            </p>

            <div className="font-mono bg-muted py-2 rounded">
              <span className="text-foreground font-bold">method</span>
              <span className="text-destructive">!=</span>
              <span className="text-accent font-semibold">"OPTIONS"</span>
            </div>
            <p className="text-muted-foreground">
              <Trans>Exclude OPTIONS method</Trans>
            </p>
          </div>

          <div className="pt-2 border-t border-border">
            <p className="text-muted-foreground text-xs">
              <Trans>Multiple conditions:</Trans>{" "}
              <code className="bg-muted px-1 py-0.5 rounded text-foreground">
                method="GET" status="2xx"
              </code>
            </p>
            <p className="text-muted-foreground text-xs mt-1">
              <Trans>Logical OR:</Trans>{" "}
              <code className="bg-muted px-1 py-0.5 rounded text-foreground">
                method="GET" or status="5xx"
              </code>
            </p>
            <p className="text-muted-foreground text-xs mt-2 flex items-center gap-1">
              <Trans>Apply:</Trans>
              <kbd className="px-1.5 py-0.5 bg-muted rounded text-[10px] border border-border text-foreground">
                ⌘
              </kbd>
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
