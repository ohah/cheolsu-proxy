import { Copy } from 'lucide-react';

import type { HttpTransaction } from '@/entities/proxy';

import { Button, Card, CardContent, CardHeader, Textarea } from '@/shared/ui';
import type { AppFormInstance } from '../context/form-context';

import { formatBody } from '../lib';
import { useMemo } from 'react';

interface TransactionResponseProps {
  transaction: HttpTransaction;
  isEditing?: boolean;
  form?: AppFormInstance;
}

export const TransactionResponse = ({ transaction, isEditing = false, form }: TransactionResponseProps) => {
  const { response } = transaction;

  if (!response) return null;

  const responseText = useMemo(() => {
    return formatBody(response.body);
  }, [response]);

  const handleCopy = () => {
    navigator.clipboard.writeText(responseText);
  };

  return (
    <Card className="gap-0">
      <CardHeader>
        <div className="flex items-center justify-end">
          <Button variant="ghost" size="sm" onClick={handleCopy}>
            <Copy className="w-4 h-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {form && isEditing ? (
          <form.Field
            name="response.data"
            children={(field) => (
              <Textarea
                value={(field.state.value as string) || ''}
                onChange={(e) => field.handleChange(e.target.value)}
                placeholder="Enter response body content..."
                className="min-h-[200px] font-mono text-xs"
              />
            )}
          />
        ) : (
          <pre className="text-xs bg-muted p-3 rounded-md overflow-auto whitespace-pre-wrap">{responseText}</pre>
        )}
      </CardContent>
    </Card>
  );
};
