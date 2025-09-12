export const TRANSACTION_DETAILS_TABS = {
  HEADERS: 'headers',
  BODY: 'body',
  RESPONSE: 'response',
} as const;

export const TRANSACTION_DETAILS_TAB_LABELS = {
  [TRANSACTION_DETAILS_TABS.HEADERS]: 'Headers',
  [TRANSACTION_DETAILS_TABS.BODY]: 'Body',
  [TRANSACTION_DETAILS_TABS.RESPONSE]: 'Response',
} as const;
