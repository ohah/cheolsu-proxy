import { Copy } from "lucide-react";
import { Trans } from "@lingui/react/macro";

import type { HttpTransaction } from "@/entities/proxy";

import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/shared/ui";
import { toast } from "sonner";

interface TransactionHeadersProps {
  transaction: HttpTransaction;
}

interface HeaderSectionProps {
  title: React.ReactNode;
  headers: Record<string, string>;
  copyLabel: string;
}

const HeaderSection = ({ title, headers, copyLabel }: HeaderSectionProps) => {
  const handleCopy = () => {
    const headersText = Object.entries(headers)
      .map(([key, value]) => `${key}: ${value}`)
      .join("\n");
    navigator.clipboard.writeText(headersText);
    toast.success(`${copyLabel} copied to clipboard`);
  };

  return (
    <Card className="gap-0">
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium">{title}</CardTitle>
          <Tooltip>
            <TooltipTrigger render={<div />}>
              <Button variant="ghost" size="sm" onClick={handleCopy}>
                <Copy className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent sideOffset={4}>
              <Trans>Copy Headers</Trans>
            </TooltipContent>
          </Tooltip>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {Object.entries(headers).map(([key, value]) => (
            <div key={key} className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground font-mono flex-1">{key}:</span>
              <span className="font-mono break-all flex-2">{value}</span>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
};

export const TransactionHeaders = ({ transaction }: TransactionHeadersProps) => {
  const { request, response } = transaction;

  if (!request?.headers && !response?.headers) return null;

  return (
    <div className="space-y-4">
      {request?.headers && (
        <HeaderSection
          title={<Trans>Request Headers</Trans>}
          headers={request.headers}
          copyLabel="Request headers"
        />
      )}
      {response?.headers && (
        <HeaderSection
          title={<Trans>Response Headers</Trans>}
          headers={response.headers}
          copyLabel="Response headers"
        />
      )}
    </div>
  );
};
