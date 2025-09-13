import { Copy } from 'lucide-react';

import type { HttpTransaction } from '@/entities/proxy';

import { Button, Card, CardContent, CardHeader, CardTitle, Textarea } from '@/shared/ui';
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
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm">Response</CardTitle>
          <div className="flex gap-2">
            <Button variant="ghost" size="sm" onClick={handleCopy}>
              <Copy className="w-4 h-4" />
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {form && isEditing ? (
          <form.Field
            name="response.data"
            children={(field) => (
              <>
                {console.log('test', field.state.value)}
                <Textarea
                  value={(field.state.value as string) || ''}
                  onChange={(e) => field.handleChange(e.target.value)}
                  placeholder="Enter response body content..."
                  className="min-h-[200px] font-mono text-xs"
                />
              </>
            )}
          />
        ) : (
          <pre className="text-xs bg-muted p-3 rounded-md overflow-auto whitespace-pre-wrap">{responseText}</pre>
        )}
      </CardContent>
    </Card>
  );
};
