import { HttpTransaction } from '@/entities/proxy';
import { HostNode, NodeType } from '../model';
import { extractHostFromRequest, extractPathFromRequest, parsePathSegments } from './utils';

const NODE_ORDER: Record<NodeType, number> = {
  host: 0,
  folder: 1,
  endpoint: 2,
} as const;

function createHostNode(
  name: string,
  path: string,
  type: NodeType,
): HostNode {
  return {
    name,
    path,
    children: new Map(),
    transactions: [],
    type,
  };
}

function createRootNode(): HostNode {
  return createHostNode('root', '', 'host');
}

function getOrCreateHostNode(
  root: HostNode,
  host: string,
): HostNode {
  if (!root.children.has(host)) {
    root.children.set(host, createHostNode(host, host, 'host'));
  }
  return root.children.get(host)!;
}

function getOrCreatePathNode(
  parentNode: HostNode,
  segment: string,
  currentPath: string,
  isLastSegment: boolean,
): HostNode {
  if (!parentNode.children.has(segment)) {
    const nodeType: NodeType = isLastSegment ? 'endpoint' : 'folder';
    parentNode.children.set(segment, createHostNode(segment, currentPath, nodeType));
  }
  return parentNode.children.get(segment)!;
}

function addTransactionToTree(
  transaction: HttpTransaction,
  root: HostNode,
  expandedPaths: Set<string>
): void {
  const request = transaction.request!;

  const host = extractHostFromRequest(request);
  const pathPart = extractPathFromRequest(request);
  const pathSegments = parsePathSegments(pathPart);

  const hostNode = getOrCreateHostNode(root, host);

  if (pathSegments.length === 0) {
    hostNode.transactions.push(transaction);
    return;
  }

  let currentNode = hostNode;

  pathSegments.forEach((segment, index) => {
    const isLastSegment = index === pathSegments.length - 1;
    const currentPath = `${host}/${pathSegments.slice(0, index + 1).join('/')}`;

    currentNode = getOrCreatePathNode(
      currentNode,
      segment,
      currentPath,
      isLastSegment,
    );

    if (isLastSegment) {
      currentNode.transactions.push(transaction);
    }
  });
}

export const buildHostTree = (
  transactions: HttpTransaction[],
  expandedPaths: Set<string>
): HostNode => {
  const root = createRootNode();

  const completeTransactions = transactions.filter(t => t.request !== null);

  completeTransactions.forEach(transaction => {
    addTransactionToTree(transaction, root, expandedPaths);
  });

  return root;
};

export function sortTreeNodes(nodes: HostNode[]): HostNode[] {
  return nodes.sort((a, b) => {
    if (a.type !== b.type) {
      return NODE_ORDER[a.type] - NODE_ORDER[b.type];
    }
    return a.name.localeCompare(b.name);
  });
}
