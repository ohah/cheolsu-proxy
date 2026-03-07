import { useEffect, useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { Blocks, Copy, Check } from "lucide-react";
import {
  Popover,
  PopoverTrigger,
  PopoverContent,
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/shared/ui";
import { cn } from "@/shared/lib";
import { getMcpServerPath } from "@/shared/api/proxy";
import { toast } from "sonner";

interface SidebarMcpProps {
  collapsed: boolean;
}

function buildMcpConfig(command: string) {
  return JSON.stringify(
    {
      mcpServers: {
        "cheolsu-proxy": { command },
      },
    },
    null,
    2,
  );
}

export const SidebarMcp = ({ collapsed }: SidebarMcpProps) => {
  const { t } = useLingui();
  const [copied, setCopied] = useState(false);
  const [mcpPath, setMcpPath] = useState("cheolsu-proxy-mcp");

  useEffect(() => {
    getMcpServerPath()
      .then(setMcpPath)
      .catch(() => {});
  }, []);

  const mcpConfig = buildMcpConfig(mcpPath);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(mcpConfig);
    setCopied(true);
    toast.success(t`MCP configuration copied to clipboard`);
    setTimeout(() => setCopied(false), 2000);
  };

  const trigger = (
    <PopoverTrigger
      className={cn(
        "flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer",
        collapsed && "justify-center",
      )}
    >
      <Blocks className="w-4 h-4 shrink-0" />
      {!collapsed && (
        <span>
          <Trans>MCP Server</Trans>
        </span>
      )}
    </PopoverTrigger>
  );

  return (
    <div className={cn("px-4 pb-2", collapsed && "flex justify-center px-0")}>
      <Popover>
        {collapsed ? (
          <Tooltip>
            <TooltipTrigger render={<div />}>{trigger}</TooltipTrigger>
            <TooltipContent side="right" sideOffset={4}>
              <Trans>MCP Server</Trans>
            </TooltipContent>
          </Tooltip>
        ) : (
          trigger
        )}
        <PopoverContent side="right" align="end" className="w-96">
          <div className="space-y-3">
            <div className="font-medium text-sm">
              <Trans>MCP Server Configuration</Trans>
            </div>
            <p className="text-xs text-muted-foreground">
              <Trans>Add this to your AI assistant&apos;s MCP configuration:</Trans>
            </p>
            <div className="relative">
              <pre className="bg-muted rounded-md p-3 text-xs overflow-x-auto whitespace-pre-wrap break-all">
                <code>{mcpConfig}</code>
              </pre>
              <button
                type="button"
                onClick={handleCopy}
                className="absolute top-2 right-2 p-1 rounded hover:bg-background/80 transition-colors cursor-pointer"
                title={t`Copy to clipboard`}
              >
                {copied ? (
                  <Check className="w-3.5 h-3.5 text-green-500" />
                ) : (
                  <Copy className="w-3.5 h-3.5 text-muted-foreground" />
                )}
              </button>
            </div>
            <p className="text-xs text-muted-foreground">
              <Trans>
                Works with Claude Code, Cursor, Claude Desktop, and other MCP-compatible clients.
              </Trans>
            </p>
          </div>
        </PopoverContent>
      </Popover>
    </div>
  );
};
