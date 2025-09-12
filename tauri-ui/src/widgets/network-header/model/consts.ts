export const HTTP_METHODS = {
  GET: 'GET',
  POST: 'POST',
  PUT: 'PUT',
  DELETE: 'DELETE',
  PATCH: 'PATCH',
  HEAD: 'HEAD',
  OPTIONS: 'OPTIONS',
  CONNECT: 'CONNECT',
  TRACE: 'TRACE',
} as const;

export const HTTP_METHOD_OPTIONS = [
  { value: HTTP_METHODS.GET, label: 'GET' },
  { value: HTTP_METHODS.POST, label: 'POST' },
  { value: HTTP_METHODS.PUT, label: 'PUT' },
  { value: HTTP_METHODS.DELETE, label: 'DELETE' },
  { value: HTTP_METHODS.PATCH, label: 'PATCH' },
  { value: HTTP_METHODS.HEAD, label: 'HEAD' },
  { value: HTTP_METHODS.OPTIONS, label: 'OPTIONS' },
  { value: HTTP_METHODS.CONNECT, label: 'CONNECT' },
  { value: HTTP_METHODS.TRACE, label: 'TRACE' },
];

export const STATUS_CODE_RANGES = {
  INFORMATIONAL: '100',
  SUCCESS: '200',
  REDIRECTION: '300',
  CLIENT_ERROR: '400',
  SERVER_ERROR: '500',
} as const;

export const STATUS_OPTIONS = [
  { value: STATUS_CODE_RANGES.INFORMATIONAL, label: '1xx Info' },
  { value: STATUS_CODE_RANGES.SUCCESS, label: '2xx Success' },
  { value: STATUS_CODE_RANGES.REDIRECTION, label: '3xx Redirect' },
  { value: STATUS_CODE_RANGES.CLIENT_ERROR, label: '4xx Client Error' },
  { value: STATUS_CODE_RANGES.SERVER_ERROR, label: '5xx Server Error' },
];
