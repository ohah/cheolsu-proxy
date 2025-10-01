import { HttpRequest, HttpTransaction } from '@/entities/proxy';

export function extractHostFromRequest(request: HttpRequest): string {
  try {
    const url = new URL(request.uri.startsWith('http') ? request.uri : `http://${request.uri}`);
    return url.hostname;
  } catch {
    const uriParts = request.uri.split('/');
    return uriParts[0] || 'unknown';
  }
}

export function extractPathFromRequest(request: HttpRequest): string {
  try {
    const url = new URL(request.uri.startsWith('http') ? request.uri : `http://${request.uri}`);
    return url.pathname;
  } catch {
    const uriParts = request.uri.split('/');
    return uriParts.length > 1 ? '/' + uriParts.slice(1).join('/') : '/';
  }
}

export function parsePathSegments(path: string): string[] {
  return path.split('/').filter(Boolean);
}

export function bodyToString(body: Uint8Array): string {
  try {
    return new TextDecoder().decode(body);
  } catch {
    return '[Binary Data]';
  }
}

export function isTransactionComplete(transaction: HttpTransaction): boolean {
  return transaction.request !== null && transaction.response !== null;
}
