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
