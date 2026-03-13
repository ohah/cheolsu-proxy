import { Trans } from "@lingui/react/macro";
import type { HttpTransaction } from "@/entities/proxy";

import { TransactionBody } from "./transaction-body";
import { TransactionResponse } from "./transaction-response";

interface TransactionBodySideBySideProps {
  transaction: HttpTransaction;
}

export const TransactionBodySideBySide = ({ transaction }: TransactionBodySideBySideProps) => {
  return (
    <div className="grid grid-cols-2 gap-4 h-full">
      <div className="flex flex-col min-h-0">
        <h3 className="text-sm font-medium text-muted-foreground mb-2">
          <Trans>Request</Trans>
        </h3>
        <div className="flex-1 min-h-0">
          <TransactionBody transaction={transaction} />
        </div>
      </div>
      <div className="flex flex-col min-h-0">
        <h3 className="text-sm font-medium text-muted-foreground mb-2">
          <Trans>Response</Trans>
        </h3>
        <div className="flex-1 min-h-0">
          <TransactionResponse transaction={transaction} />
        </div>
      </div>
    </div>
  );
};
