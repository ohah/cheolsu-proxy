import type { HttpTransaction } from '@/entities/proxy';

import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/shared/ui';

import { TransactionHeader } from './transaction-header';
import { TransactionProperties } from './transaction-properties';
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

  if (!request || !response) {
    return null;
  }

  return (
    <div className="h-full bg-card border-l border-border flex flex-col">
      <TransactionHeader
        transaction={transaction}
        clearSelectedTransaction={clearSelectedTransaction}
        isEditing={isEditing}
        onStartEdit={startEditing}
        onCancelEdit={cancelEditing}
        onSaveEdit={saveChanges}
        form={form}
      />

      <div className="flex-1 overflow-auto p-4">
        <div className="space-y-4">
          <TransactionProperties transaction={transaction} />

          <Tabs value={activeTab} onValueChange={onTabChange}>
            <TabsList className="grid w-full" style={{ gridTemplateColumns: `repeat(${tabs.length}, 1fr)` }}>
              {tabs.map((tab) => (
                <TabsTrigger key={tab} value={tab}>
                  {TRANSACTION_DETAILS_TAB_LABELS[tab]}
                </TabsTrigger>
              ))}
            </TabsList>

            <TabsContent value={TRANSACTION_DETAILS_TABS.HEADERS} className="mt-4">
              <TransactionHeaders transaction={transaction} isEditing={isEditing} form={form as any} />
            </TabsContent>

            <TabsContent value={TRANSACTION_DETAILS_TABS.BODY} className="mt-4">
              <TransactionBody transaction={transaction} isEditing={isEditing} form={form as any} />
            </TabsContent>

            <TabsContent value={TRANSACTION_DETAILS_TABS.RESPONSE} className="mt-4">
              <TransactionResponse transaction={transaction} isEditing={isEditing} form={form as any} />
            </TabsContent>
          </Tabs>
        </div>
      </div>
    </div>
  );
}
