export interface ParsedQuery {
  methods: string[];
  excludeMethods: string[];
  status: string[];
  excludeStatus: string[];
  urls: string[];
  excludeUrls: string[];
  operator: "and" | "or";
}

/**
 * 예:
 * - method="GET,POST" status="2xx"
 * - method!="GET" status="5xx"
 * - method="GET" or status="2xx"
 * - url|="payhere" url|="hegeg" (둘 다 포함)
 * - url|="payhere,hegeg" (둘 다 포함, 콤마로 구분)
 *
 * 연산자:
 * - = : 포함 (equals/contains)
 * - |= : 포함 (contains)
 * - != : 제외 (not equals)
 * - or : 논리 OR (하나라도 만족)
 * - and : 논리 AND (모두 만족, 기본값)
 */
export function parseFilterQuery(query: string): ParsedQuery {
  const result: ParsedQuery = {
    methods: [],
    excludeMethods: [],
    status: [],
    excludeStatus: [],
    urls: [],
    excludeUrls: [],
    operator: "and",
  };

  // or 연산자 감지
  if (/\bor\b/i.test(query)) {
    result.operator = "or";
  }

  // key(operator)"value" 패턴 추출
  const regex = /(\w+)\s*(!?=|\|=)\s*"([^"]*)"/g;
  let match;

  while ((match = regex.exec(query)) !== null) {
    const [, key, operator, value] = match;
    const isExclude = operator === "!=";

    switch (key.toLowerCase()) {
      case "method":
      case "methods":
        const methods = value
          .split(",")
          .map((m) => m.trim().toUpperCase())
          .filter(Boolean);

        if (isExclude) {
          result.excludeMethods.push(...methods);
        } else {
          result.methods.push(...methods);
        }
        break;

      case "status":
        const statuses = value
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean);

        if (isExclude) {
          result.excludeStatus.push(...statuses);
        } else {
          result.status.push(...statuses);
        }
        break;

      case "url":
        // 콤마로 구분된 여러 URL 조건 지원
        const urlParts = value
          .split(",")
          .map((u) => u.trim())
          .filter(Boolean);

        if (isExclude) {
          result.excludeUrls.push(...urlParts);
        } else {
          result.urls.push(...urlParts);
        }
        break;
    }
  }

  return result;
}
