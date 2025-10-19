export const HTTP_METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS', 'CONNECT', 'TRACE'] as const;

export const STATUS_CODES = [
  { code: '1xx', detail: 'Informational' },
  { code: '2xx', detail: 'Success' },
  { code: '3xx', detail: 'Redirection' },
  { code: '4xx', detail: 'Client Error' },
  { code: '5xx', detail: 'Server Error' },
  { code: '200', detail: 'OK' },
  { code: '201', detail: 'Created' },
  { code: '204', detail: 'No Content' },
  { code: '400', detail: 'Bad Request' },
  { code: '401', detail: 'Unauthorized' },
  { code: '403', detail: 'Forbidden' },
  { code: '404', detail: 'Not Found' },
  { code: '500', detail: 'Internal Server Error' },
  { code: '502', detail: 'Bad Gateway' },
  { code: '503', detail: 'Service Unavailable' },
] as const;

export const FILTER_KEYWORDS = ['method', 'methods', 'status', 'url'] as const;
export const LOGICAL_OPERATORS = ['and', 'or'] as const;
export const COMPARISON_OPERATORS = ['=', '|=', '|~', '!=', '!~'] as const;
