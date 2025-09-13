import { Edit, X } from 'lucide-react';

import type { HttpTransaction } from '@/entities/proxy';

import { getStatusColor } from '@/widgets/network-table';

import { Badge, Button } from '@/shared/ui';

interface TransactionHeaderProps {
  transaction: HttpTransaction;
  clearSelectedTransaction: () => void;
}

export const TransactionHeader = ({ transaction, clearSelectedTransaction }: TransactionHeaderProps) => {
  const { response } = transaction;

  if (!response) return null;

  return (
    <div className="flex items-center justify-between p-4 border-b border-border">
      <div className="flex items-center gap-2">
        <h2 className="font-semibold text-card-foreground">Request Details</h2>
        <Badge variant="outline" className={`text-xs ${getStatusColor(response.status)}`}>
          {response.status}
        </Badge>
      </div>
      <div className="flex items-center">
        {/* TODO: settings @ohah */}
        <Button variant="ghost" size="sm" onClick={() => {}}>
          <Edit className="w-4 h-4" />
        </Button>
        <Button variant="ghost" size="sm" onClick={clearSelectedTransaction}>
          <X className="w-4 h-4" />
        </Button>
      </div>
    </div>
  );
};
