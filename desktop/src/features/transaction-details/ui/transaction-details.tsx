import type { HttpTransaction } from "@/entities/proxy";

import { ScrollArea, Tabs, TabsContent, TabsList, TabsTrigger } from "@/shared/ui";

import { TransactionHeader } from "./transaction-header";
import { TransactionHeaders } from "./transaction-headers";
import { TransactionBody } from "./transaction-body";
import { TransactionResponse } from "./transaction-response";
import { TransactionBodySideBySide } from "./transaction-body-side-by-side";
import { TransactionTiming } from "./transaction-timing";
import { TransactionValidation } from "./transaction-validation";
import { TransactionCertificate } from "./transaction-certificate";

import { useTransactionTabs } from "../hooks";
import {
  TRANSACTION_DETAILS_TAB_LABELS,
  TRANSACTION_DETAILS_TABS,
  TRANSACTION_DETAILS_BOTTOM_TAB_LABELS,
  TRANSACTION_DETAILS_BOTTOM_TABS,
} from "../model";
import type { DetailsPanelLayout } from "@/shared/stores/app-settings-store";

interface TransactionDetailsProps {
  transaction: HttpTransaction;
  clearSelectedTransaction: () => void;
  layout?: DetailsPanelLayout;
}

export function TransactionDetails({
  transaction,
  clearSelectedTransaction,
  layout = "right",
}: TransactionDetailsProps) {
  const { request, response } = transaction;

  const { activeTab, tabs, onTabChange } = useTransactionTabs(layout);

  if (!request || !response) {
    return null;
  }

  const isBottom = layout === "bottom";
  const tabLabels = isBottom
    ? TRANSACTION_DETAILS_BOTTOM_TAB_LABELS
    : TRANSACTION_DETAILS_TAB_LABELS;

  const hasCert = !!transaction.server_cert;
  const filteredTabs = isBottom
    ? tabs
    : tabs.filter((tab) => tab !== TRANSACTION_DETAILS_TABS.CERTIFICATE || hasCert);

  return (
    <div className="h-full bg-card flex flex-col select-text">
      <TransactionHeader
        transaction={transaction}
        clearSelectedTransaction={clearSelectedTransaction}
      />

      <ScrollArea className="flex-1 overflow-auto p-4 [&_[data-slot=scroll-area-viewport]]:!flex [&_[data-slot=scroll-area-viewport]>div]:!w-full">
        <div className="h-full w-full">
          <Tabs value={activeTab} onValueChange={onTabChange} className="h-full flex flex-col">
            <TabsList
              className="grid w-full flex-shrink-0"
              style={{
                gridTemplateColumns: `repeat(${filteredTabs.length}, 1fr)`,
              }}
            >
              {filteredTabs.map((tab) => (
                <TabsTrigger key={tab} value={tab}>
                  {tabLabels[tab as keyof typeof tabLabels]}
                </TabsTrigger>
              ))}
            </TabsList>

            {isBottom ? (
              <>
                <TabsContent
                  value={TRANSACTION_DETAILS_BOTTOM_TABS.HEADERS}
                  className="flex-1 mt-4"
                >
                  <TransactionHeaders transaction={transaction} sideBySide />
                </TabsContent>

                <TabsContent value={TRANSACTION_DETAILS_BOTTOM_TABS.BODY} className="flex-1 mt-4">
                  <TransactionBodySideBySide transaction={transaction} />
                </TabsContent>
              </>
            ) : (
              <>
                <TabsContent value={TRANSACTION_DETAILS_TABS.HEADERS} className="flex-1 mt-4">
                  <TransactionHeaders transaction={transaction} />
                </TabsContent>

                <TabsContent value={TRANSACTION_DETAILS_TABS.BODY} className="flex-1 mt-4">
                  <TransactionBody transaction={transaction} />
                </TabsContent>

                <TabsContent value={TRANSACTION_DETAILS_TABS.RESPONSE} className="flex-1 mt-4">
                  <TransactionResponse transaction={transaction} />
                </TabsContent>

                <TabsContent value={TRANSACTION_DETAILS_TABS.TIMING} className="flex-1 mt-4">
                  <TransactionTiming timing={response.timing} />
                </TabsContent>

                <TabsContent value={TRANSACTION_DETAILS_TABS.VALIDATION} className="flex-1 mt-4">
                  <TransactionValidation validations={transaction.validations} />
                </TabsContent>

                {hasCert && (
                  <TabsContent value={TRANSACTION_DETAILS_TABS.CERTIFICATE} className="flex-1 mt-4">
                    <TransactionCertificate serverCert={transaction.server_cert} />
                  </TabsContent>
                )}
              </>
            )}
          </Tabs>
        </div>
      </ScrollArea>
    </div>
  );
}
