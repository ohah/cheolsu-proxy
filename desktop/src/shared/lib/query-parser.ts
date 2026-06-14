export interface ParsedQuery {
  methods: string[];
  excludeMethods: string[];
  status: string[];
  excludeStatus: string[];
  urls: string[];
  excludeUrls: string[];
  clients: string[];
  excludeClients: string[];
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
 * 따옴표:
 * - 큰따옴표: method="GET"
 * - 작은따옴표: method='GET'
 * - 백틱: method=`GET`
 * - 이스케이프: url|="path/with\"quote"
 *
 * 연산자:
 * - = : 포함 (equals/contains)
 * - |= : 포함 (contains)
 * - != : 제외 (not equals)
 * - or : 논리 OR (하나라도 만족)
 * - and : 논리 AND (모두 만족, 기본값)
 */
/**
 * 이스케이프되지 않은 콤마(,)로 값을 분리하고, 각 조각의 이스케이프(\, \" \' \` \\)를 해제한다.
 * 값 안에 콤마가 포함돼도(예: URL의 쿼리 파라미터) 여러 조건으로 잘못 분해되지 않는다.
 */
function splitEscapedValues(raw: string): string[] {
  const parts: string[] = [];
  let current = "";
  for (let i = 0; i < raw.length; i += 1) {
    const ch = raw[i];
    if (ch === "\\" && i + 1 < raw.length) {
      const next = raw[i + 1];
      // 알려진 이스케이프(\, \" \' \` \\)만 해제하고, 그 외(\d 등 정규식)는 백슬래시를 보존한다
      if (next === "," || next === '"' || next === "'" || next === "`" || next === "\\") {
        current += next;
      } else {
        current += ch + next;
      }
      i += 1;
    } else if (ch === ",") {
      parts.push(current);
      current = "";
    } else {
      current += ch;
    }
  }
  parts.push(current);
  return parts;
}

export function parseFilterQuery(query: string): ParsedQuery {
  const result: ParsedQuery = {
    methods: [],
    excludeMethods: [],
    status: [],
    excludeStatus: [],
    urls: [],
    excludeUrls: [],
    clients: [],
    excludeClients: [],
    operator: "and",
  };

  // or 연산자 감지
  if (/\bor\b/i.test(query)) {
    result.operator = "or";
  }

  // key(operator)quote(value)quote 패턴 추출
  // 지원 따옴표: " ' `
  // 이스케이프된 따옴표 지원: \" \' \`
  const regex =
    /(\w+)\s*(!?=|\|=)\s*(?:"((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)'|`((?:[^`\\]|\\.)*)`)/g;
  let match;

  while ((match = regex.exec(query)) !== null) {
    const [, key, operator] = match;
    // 3개 캡처 그룹 중 매칭된 것 사용
    const rawValue = match[3] ?? match[4] ?? match[5] ?? "";
    // 이스케이프되지 않은 콤마로 값을 분리하고 각 조각의 이스케이프를 해제한다(값 내부 콤마 보존)
    const values = splitEscapedValues(rawValue);
    const isExclude = operator === "!=";

    switch (key.toLowerCase()) {
      case "method":
      case "methods": {
        const methods = values.map((m) => m.trim().toUpperCase()).filter(Boolean);

        if (isExclude) {
          result.excludeMethods.push(...methods);
        } else {
          result.methods.push(...methods);
        }
        break;
      }

      case "status": {
        const statuses = values.map((s) => s.trim()).filter(Boolean);

        if (isExclude) {
          result.excludeStatus.push(...statuses);
        } else {
          result.status.push(...statuses);
        }
        break;
      }

      case "url": {
        // 콤마로 구분된 여러 URL 조건 지원
        const urlParts = values.map((u) => u.trim()).filter(Boolean);

        if (isExclude) {
          result.excludeUrls.push(...urlParts);
        } else {
          result.urls.push(...urlParts);
        }
        break;
      }

      case "client": {
        const clientParts = values.map((c) => c.trim()).filter(Boolean);

        if (isExclude) {
          result.excludeClients.push(...clientParts);
        } else {
          result.clients.push(...clientParts);
        }
        break;
      }
    }
  }

  return result;
}
