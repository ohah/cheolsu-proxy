import type { RequestInfo } from '../../../entities/request';

// HTTP 메서드 필터 옵션
export const HTTP_METHOD_OPTIONS = [
  'POST',
  'GET',
  'PUT',
  'DELETE',
  'PATCH',
  'HEAD',
  'OPTIONS',
  'CONNECT',
  'TRACE',
  'OTHERS',
] as const;

export type HttpMethod = (typeof HTTP_METHOD_OPTIONS)[number];

// 필터링 로직
export const filterRequest = (method: string, filters: string[]): boolean => {
  return (
    filters.includes(method) || (!HTTP_METHOD_OPTIONS.includes(method as HttpMethod) && filters.includes('OTHERS'))
  );
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

export const getStatusColor = (status: number): string => {
  if (status >= 200 && status < 300) return 'text-green-600';
  if (status >= 300 && status < 400) return 'text-blue-600';
  if (status >= 400 && status < 500) return 'text-yellow-600';
  if (status >= 500) return 'text-red-600';
  return 'text-gray-600';
};

// 바이트 포맷팅 유틸리티
const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${Number.parseFloat((bytes / k ** i).toFixed(1))} ${sizes[i]}`;
};
