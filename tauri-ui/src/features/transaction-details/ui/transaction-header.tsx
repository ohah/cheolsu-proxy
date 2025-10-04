import { Edit, X, Save, XCircle, Trash2, Code } from 'lucide-react';

import type { HttpTransaction } from '@/entities/proxy';

import { getStatusColor } from '@/entities/transaction';
import type { AppFormInstance } from '../context/form-context';

import { Badge, Button, Input } from '@/shared/ui';
import { generateCurlCommand } from '../lib';
import { toast } from 'sonner';

interface TransactionHeaderProps {
  transaction: HttpTransaction;
  clearSelectedTransaction: () => void;
  isEditing: boolean;
  onStartEdit: () => void;
  onCancelEdit: () => void;
  onSaveEdit: () => void;
  onDeleteSession: () => void;
  form?: AppFormInstance;
}

export const TransactionHeader = ({
  transaction,
  clearSelectedTransaction,
  isEditing,
  onStartEdit,
  onCancelEdit,
  onSaveEdit,
  onDeleteSession,
  form,
}: TransactionHeaderProps) => {
  const { response } = transaction;

  if (!response) return null;

  const handleCopyCurl = () => {
    const curlCommand = generateCurlCommand(transaction);
    navigator.clipboard.writeText(curlCommand);
    toast.success('Curl command copied to clipboard');
  };

  return (
    <div className="flex items-center justify-between p-4 border-b border-border">
      <div className="flex items-center gap-2">
        <h2 className="font-semibold text-card-foreground">Request Details</h2>
        {form && isEditing ? (
          <form.Field
            name="response.status"
            children={(field: any) => (
              <Input
                type="text"
                pattern="[0-9]*"
                maxLength={3}
                value={field.state.value || response.status}
                onChange={(e) => field.handleChange(parseInt(e.target.value) || 200)}
                className={`w-18 h-6 text-xs text-center font-mono ${getStatusColor(field.state.value || response.status)}`}
              />
            )}
          />
        ) : (
          <Badge variant="outline" className={`text-xs ${getStatusColor(response.status)}`}>
            {response.status}
          </Badge>
        )}
      </div>
      <div className="flex items-center gap-2">
        <Button variant="ghost" size="sm" onClick={handleCopyCurl}>
          <Code className="w-4 h-4" />
        </Button>
        {isEditing ? (
          <>
            <Button variant="ghost" size="sm" onClick={onSaveEdit}>
              <Save className="w-4 h-4" />
            </Button>
            <Button variant="ghost" size="sm" onClick={onCancelEdit}>
              <XCircle className="w-4 h-4" />
            </Button>
          </>
        ) : (
          <Button variant="ghost" size="sm" onClick={onStartEdit}>
            <Edit className="w-4 h-4" />
          </Button>
        )}
        <Button variant="ghost" size="sm" onClick={onDeleteSession}>
          <Trash2 className="w-4 h-4" />
        </Button>
        <Button variant="ghost" size="sm" onClick={clearSelectedTransaction}>
          <X className="w-4 h-4" />
        </Button>
      </div>
    </div>
  );
};
