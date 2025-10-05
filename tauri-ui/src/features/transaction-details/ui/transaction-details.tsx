import type { HttpTransaction } from '@/entities/proxy';

import { ScrollArea, Tabs, TabsContent, TabsList, TabsTrigger } from '@/shared/ui';
import { useSessionStore } from '@/shared/stores';

import { TransactionHeader } from './transaction-header';
import { TransactionHeaders } from './transaction-headers';
import { TransactionBody } from './transaction-body';
import { TransactionResponse } from './transaction-response';

import { useTransactionTabs, useTransactionEdit } from '../hooks';
import { TRANSACTION_DETAILS_TAB_LABELS, TRANSACTION_DETAILS_TABS } from '../model';

interface TransactionDetailsProps {
  transaction: HttpTransaction;
  clearSelectedTransaction: () => void;
}

export function TransactionDetails({ transaction, clearSelectedTransaction }: TransactionDetailsProps) {
  const { request, response } = transaction;

  const { activeTab, tabs, onTabChange } = useTransactionTabs();
  const { isEditing, form, startEditing, cancelEditing, saveChanges } = useTransactionEdit(transaction);
  const deleteSessionByUrl = useSessionStore((state) => state.deleteSessionByUrl);

  const handleDeleteSession = () => {
    if (request?.id) {
      deleteSessionByUrl(request.uri);
      clearSelectedTransaction();
    }
  };

  if (!request || !response) {
    return null;
  }

  return (
    <div className="h-full bg-card flex flex-col">
      <TransactionHeader
        transaction={transaction}
        clearSelectedTransaction={clearSelectedTransaction}
        isEditing={isEditing}
        onStartEdit={startEditing}
        onCancelEdit={cancelEditing}
        onSaveEdit={saveChanges}
        onDeleteSession={handleDeleteSession}
        form={form}
      />

      <ScrollArea className="flex-1 overflow-auto p-4 [&>div>div]:!flex">
        <div className="h-full w-full">
          <Tabs value={activeTab} onValueChange={onTabChange} className="h-full flex flex-col">
            <TabsList
              className="grid w-full flex-shrink-0"
              style={{ gridTemplateColumns: `repeat(${tabs.length}, 1fr)` }}
            >
              {tabs.map((tab) => (
                <TabsTrigger key={tab} value={tab}>
                  {TRANSACTION_DETAILS_TAB_LABELS[tab]}
                </TabsTrigger>
              ))}
            </TabsList>

            <TabsContent value={TRANSACTION_DETAILS_TABS.HEADERS} className="flex-1 mt-4">
              <TransactionHeaders transaction={transaction} isEditing={isEditing} form={form} />
            </TabsContent>

            <TabsContent value={TRANSACTION_DETAILS_TABS.BODY} className="flex-1 mt-4">
              <TransactionBody request={request} isEditing={isEditing} form={form} />
            </TabsContent>

            <TabsContent value={TRANSACTION_DETAILS_TABS.RESPONSE} className="flex-1 mt-4">
              <TransactionResponse response={response} isEditing={isEditing} form={form} />
            </TabsContent>
          </Tabs>
        </div>
      </ScrollArea>
    </div>
  );
}
