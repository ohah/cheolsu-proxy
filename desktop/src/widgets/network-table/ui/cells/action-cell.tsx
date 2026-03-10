import { memo } from "react";
import { Trash2 } from "lucide-react";
import { Trans } from "@lingui/react/macro";

import { Button, Tooltip, TooltipTrigger, TooltipContent } from "@/shared/ui";

interface ActionCellProps {
  onDelete?: () => void;
}

export const ActionCell = memo<ActionCellProps>(({ onDelete }) => {
  return (
    <div className="col-span-1">
      <Tooltip>
        <TooltipTrigger render={<div />}>
          <Button variant="outline" size="sm" onClick={onDelete}>
            <Trash2 className="w-4 h-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent sideOffset={4}>
          <Trans>Delete Transaction</Trans>
        </TooltipContent>
      </Tooltip>
    </div>
  );
});

ActionCell.displayName = "ActionCell";
