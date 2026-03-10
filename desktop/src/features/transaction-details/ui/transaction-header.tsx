import { X, Code } from "lucide-react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";

import type { HttpTransaction } from "@/entities/proxy";

import { getStatusColor } from "@/entities/transaction";

import { Badge, Button, Tooltip, TooltipTrigger, TooltipContent } from "@/shared/ui";
import { generateCurlCommand } from "../lib";
import { toast } from "sonner";
import { ReplayDialog } from "./replay-dialog";

interface TransactionHeaderProps {
  transaction: HttpTransaction;
  clearSelectedTransaction: () => void;
}

export const TransactionHeader = ({
  transaction,
  clearSelectedTransaction,
}: TransactionHeaderProps) => {
  const { t } = useLingui();
  const { response } = transaction;

  if (!response) return null;

  const handleCopyCurl = () => {
    const curlCommand = generateCurlCommand(transaction);
    navigator.clipboard.writeText(curlCommand);
    toast.success(t`Curl command copied to clipboard`);
  };

  return (
    <div className="flex items-center justify-between p-4 border-b border-border">
      <div className="flex items-center gap-2">
        <h2 className="font-semibold text-card-foreground">
          <Trans>Request Details</Trans>
        </h2>
        <Badge variant="outline" className={`text-xs ${getStatusColor(response.status)}`}>
          {response.status}
        </Badge>
      </div>
      <div className="flex items-center gap-2">
        <ReplayDialog transaction={transaction} />
        <Tooltip>
          <TooltipTrigger render={<div />}>
            <Button variant="ghost" size="sm" onClick={handleCopyCurl}>
              <Code className="w-4 h-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent sideOffset={4}>
            <Trans>Copy as cURL</Trans>
          </TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger render={<div />}>
            <Button variant="ghost" size="sm" onClick={clearSelectedTransaction}>
              <X className="w-4 h-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent sideOffset={4}>
            <Trans>Close</Trans>
          </TooltipContent>
        </Tooltip>
      </div>
    </div>
  );
};
