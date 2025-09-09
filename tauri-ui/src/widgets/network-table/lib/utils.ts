import type { RequestInfo } from '@/entities/proxy';

import { HTTP_METHOD_OPTIONS, HttpMethod } from '../model';

// 상태 색상 유틸리티
export const getStatusColor = (status: number): string => {
  if (status >= 200 && status < 300) return 'bg-green-100 text-green-800 border-green-200';
  if (status >= 300 && status < 400) return 'bg-blue-100 text-blue-800 border-blue-200';
  if (status >= 400 && status < 500) return 'bg-yellow-100 text-yellow-800 border-yellow-200';
  if (status >= 500) return 'bg-red-100 text-red-800 border-red-200';
  return 'bg-gray-100 text-gray-800 border-gray-200';
};

export const getMethodColor = (method: string) => {
  switch (method) {
    case 'GET':
      return 'bg-blue-100 text-blue-800 border-blue-200';
    case 'POST':
      return 'bg-green-100 text-green-800 border-green-200';
    case 'PUT':
      return 'bg-yellow-100 text-yellow-800 border-yellow-200';
    case 'DELETE':
      return 'bg-red-100 text-red-800 border-red-200';
    default:
      return 'bg-gray-100 text-gray-800 border-gray-200';
  }
};

// 바이트 포맷팅 유틸리티
const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${Number.parseFloat((bytes / k ** i).toFixed(1))} ${sizes[i]}`;
};

// 요청 정보 유틸리티 함수들
export const getRequestSize = (request: RequestInfo): string => {
  if (!request.request) return '0 B';
  const size = request.request.body?.length || 0;
  return formatBytes(size);
};

export const getResponseSize = (request: RequestInfo): string => {
  if (!request.response) return '0 B';
  const size = request.response.body?.length || 0;
  return formatBytes(size);
};

// 필터링 로직
export const filterRequest = (method: string, filters: string[]): boolean => {
  return (
    filters.includes(method) || (!HTTP_METHOD_OPTIONS.includes(method as HttpMethod) && filters.includes('OTHERS'))
  );
};

export const getAuthority = (uri: string) => {
  try {
    const url = new URL(uri);
    return `${url.hostname}${url.port ? `:${url.port}` : ''}`;
  } catch (e) {
    return uri.split('/')[0] || uri;
  }
};
