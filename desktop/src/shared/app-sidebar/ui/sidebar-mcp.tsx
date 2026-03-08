import { useEffect, useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { Blocks, Copy, Check, Terminal } from "lucide-react";
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

function buildClaudeCommand(command: string) {
  return `claude mcp add cheolsu-proxy -- ${command}`;
}

export const SidebarMcp = ({ collapsed }: SidebarMcpProps) => {
  const { t } = useLingui();
  const [copiedJson, setCopiedJson] = useState(false);
  const [copiedCmd, setCopiedCmd] = useState(false);
  const [mcpPath, setMcpPath] = useState("cheolsu-proxy-mcp");

  useEffect(() => {
    getMcpServerPath()
      .then(setMcpPath)
      .catch(() => {});
  }, []);

  const mcpConfig = buildMcpConfig(mcpPath);
  const claudeCmd = buildClaudeCommand(mcpPath);

  const handleCopyJson = async () => {
    await navigator.clipboard.writeText(mcpConfig);
    setCopiedJson(true);
    toast.success(t`MCP configuration copied to clipboard`);
    setTimeout(() => setCopiedJson(false), 2000);
  };

  const handleCopyCmd = async () => {
    await navigator.clipboard.writeText(claudeCmd);
    setCopiedCmd(true);
    toast.success(t`Command copied to clipboard`);
    setTimeout(() => setCopiedCmd(false), 2000);
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

            <div className="space-y-1.5">
              <p className="text-xs font-medium text-foreground">
                <Trans>Claude Code (CLI)</Trans>
              </p>
              <div className="relative">
                <pre className="bg-muted rounded-md p-3 pr-8 text-xs overflow-x-auto whitespace-pre-wrap break-all">
                  <code>{claudeCmd}</code>
                </pre>
                <button
                  type="button"
                  onClick={handleCopyCmd}
                  className="absolute top-2 right-2 p-1 rounded hover:bg-background/80 transition-colors cursor-pointer"
                  title={t`Copy to clipboard`}
                >
                  {copiedCmd ? (
                    <Check className="w-3.5 h-3.5 text-green-500" />
                  ) : (
                    <Terminal className="w-3.5 h-3.5 text-muted-foreground" />
                  )}
                </button>
              </div>
            </div>

            <div className="space-y-1.5">
              <p className="text-xs font-medium text-foreground">
                <Trans>JSON Configuration</Trans>
              </p>
              <p className="text-xs text-muted-foreground">
                <Trans>For Cursor, Claude Desktop, and other MCP-compatible clients:</Trans>
              </p>
              <div className="relative">
                <pre className="bg-muted rounded-md p-3 pr-8 text-xs overflow-x-auto whitespace-pre-wrap break-all">
                  <code>{mcpConfig}</code>
                </pre>
                <button
                  type="button"
                  onClick={handleCopyJson}
                  className="absolute top-2 right-2 p-1 rounded hover:bg-background/80 transition-colors cursor-pointer"
                  title={t`Copy to clipboard`}
                >
                  {copiedJson ? (
                    <Check className="w-3.5 h-3.5 text-green-500" />
                  ) : (
                    <Copy className="w-3.5 h-3.5 text-muted-foreground" />
                  )}
                </button>
              </div>
            </div>
          </div>
        </PopoverContent>
      </Popover>
    </div>
  );
};
