import { useState, useMemo } from 'react';

import { HttpTransaction } from '@/entities/proxy';

import { buildHostTree } from '../lib/tree-builder';

export const useHostTree = (transactions: HttpTransaction[]) => {
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());

  const tree = useMemo(() => {
    return buildHostTree(transactions, expandedPaths);
  }, [transactions, expandedPaths]);

  const toggleExpanded = (path: string) => {
    const newExpanded = new Set(expandedPaths);
    if (newExpanded.has(path)) {
      newExpanded.delete(path);
    } else {
      newExpanded.add(path);
    }
    setExpandedPaths(newExpanded);
  };

  return {
    tree,
    expandedPaths,
    toggleExpanded
  };
};
