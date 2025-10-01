import { Globe, Folder, File } from 'lucide-react';

import { HttpTransaction } from '@/entities/proxy';

import { HostNode } from '../model';

export const getNodeIcon = (node: HostNode) => {
  switch (node.type) {
    case 'host':
      return <Globe className="h-3 w-3 text-blue-500 flex-shrink-0" />;
    case 'folder':
      return <Folder className="h-3 w-3 text-amber-500 flex-shrink-0" />;
    case 'endpoint':
      return <File className="h-3 w-3 text-muted-foreground flex-shrink-0" />;
    default:
      return <File className="h-3 w-3 text-muted-foreground flex-shrink-0" />;
  }
};

export const getStatusDisplay = (transaction: HttpTransaction): string => {
  if (transaction.response) {
    return transaction.response.status.toString();
  }
  if (transaction.request) {
    return 'Pending';
  }
  return 'Unknown';
};

export const getStatusColor = (transaction: HttpTransaction): string => {
  if (!transaction.response) return 'bg-muted text-muted-foreground';

  const status = transaction.response.status;
  if (status >= 200 && status < 300) {
    return 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400';
  }
  if (status >= 400) {
    return 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400';
  }
  return 'bg-muted text-muted-foreground';
};
