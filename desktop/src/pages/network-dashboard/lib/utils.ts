import type { HttpTransaction } from "@/entities/proxy";

const getStatusRange = (status: number): number => {
  return Math.floor(status / 100) * 100;
};

const matchesStatusFilter = (
  status: number,
  statusFilters: string[],
  excludeFilters: string[],
): boolean => {
  // 제외 필터 먼저 체크
  const isExcluded = excludeFilters.some((filter) => {
    if (filter.endsWith("xx")) {
      const range = parseInt(filter) * 100;
      return getStatusRange(status) === range;
    }
    return status === parseInt(filter);
  });

  if (isExcluded) return false;

  // 포함 필터가 없으면 모두 통과
  if (statusFilters.length === 0) return true;

  // 포함 필터 체크
  return statusFilters.some((filter) => {
    if (filter.endsWith("xx")) {
      const range = parseInt(filter) * 100;
      return getStatusRange(status) === range;
    }
    return status === parseInt(filter);
  });
};

const matchesMethodFilter = (
  method: string,
  methodFilter: string[],
  excludeFilter: string[],
): boolean => {
  const upperMethod = method.toUpperCase();

  // 제외 필터 먼저 체크
  if (excludeFilter.includes(upperMethod)) return false;

  // 포함 필터가 없으면 모두 통과
  if (methodFilter.length === 0) return true;

  // 포함 필터 체크
  return methodFilter.includes(upperMethod);
};

const matchesPathFilter = (
  path: string,
  pathFilters: string[],
  excludeFilters: string[],
): boolean => {
  const lowerPath = path.toLowerCase();

  // 제외 필터 체크 - 하나라도 걸리면 제외
  const isExcluded = excludeFilters.some((filter) => {
    if (filter.includes("*")) {
      const regex = new RegExp(filter.replace(/\*/g, ".*"), "i");
      return regex.test(path);
    }
    return lowerPath.includes(filter.toLowerCase());
  });

  if (isExcluded) return false;

  // 포함 필터가 없으면 모두 통과
  if (pathFilters.length === 0) return true;

  // 포함 필터 체크 - 모두 만족해야 함 (AND 조건)
  return pathFilters.every((filter) => {
    if (filter.includes("*")) {
      const regex = new RegExp(filter.replace(/\*/g, ".*"), "i");
      return regex.test(path);
    }
    return lowerPath.includes(filter.toLowerCase());
  });
};

const matchesClientFilter = (
  clientAddr: string | undefined,
  proxyAuthUser: string | undefined,
  clientFilters: string[],
  excludeFilters: string[],
  clientTags: Record<string, string> = {},
): boolean => {
  const addr = clientAddr ?? "";
  // IP에서 포트 제거 + IPv6 대괄호 제거
  // "[::1]:54321" → "::1", "192.168.1.1:54321" → "192.168.1.1", "::1" → "::1"
  const cleanIp = addr.replace(/\]?:\d+$/, "").replace(/^\[|\]$/g, "");
  const tag = clientTags[cleanIp] ?? "";
  const user = proxyAuthUser ?? "";

  // 제외 필터 체크
  const isExcluded = excludeFilters.some((filter) => {
    const lowerFilter = filter.toLowerCase();
    return (
      cleanIp.includes(lowerFilter) ||
      tag.toLowerCase().includes(lowerFilter) ||
      user.toLowerCase().includes(lowerFilter)
    );
  });

  if (isExcluded) return false;

  // 포함 필터가 없으면 모두 통과
  if (clientFilters.length === 0) return true;

  // 포함 필터 체크 (IP, 태그, 사용자명 중 하나라도 매칭)
  return clientFilters.some((filter) => {
    const lowerFilter = filter.toLowerCase();
    return (
      cleanIp.includes(lowerFilter) ||
      tag.toLowerCase().includes(lowerFilter) ||
      user.toLowerCase().includes(lowerFilter)
    );
  });
};

export const getFilteredTransactions = (
  transactions: HttpTransaction[],
  statusFilter: string[],
  methodFilter: string[],
  pathFilters: string[],
  excludeStatus: string[] = [],
  excludeMethods: string[] = [],
  excludeUrls: string[] = [],
  clientFilters: string[] = [],
  excludeClients: string[] = [],
  clientTags: Record<string, string> = {},
  operator: "and" | "or" = "and",
): HttpTransaction[] => {
  if (
    statusFilter.length === 0 &&
    methodFilter.length === 0 &&
    pathFilters.length === 0 &&
    excludeStatus.length === 0 &&
    excludeMethods.length === 0 &&
    excludeUrls.length === 0 &&
    clientFilters.length === 0 &&
    excludeClients.length === 0
  ) {
    return transactions;
  }

  return transactions.filter((transaction) => {
    const status = transaction.response?.status ?? 0;
    const method = transaction.request?.method ?? "";
    const path = transaction.request?.uri ?? "";
    const clientAddr = transaction.request?.client_addr;
    const proxyAuthUser = transaction.request?.proxy_auth_user;

    const statusMatch = matchesStatusFilter(status, statusFilter, excludeStatus);
    const methodMatch = matchesMethodFilter(method, methodFilter, excludeMethods);
    const pathMatch = matchesPathFilter(path, pathFilters, excludeUrls);
    const clientMatch = matchesClientFilter(
      clientAddr,
      proxyAuthUser,
      clientFilters,
      excludeClients,
      clientTags,
    );

    if (operator === "or") {
      const hasIncludeFilter =
        statusFilter.length > 0 ||
        methodFilter.length > 0 ||
        pathFilters.length > 0 ||
        clientFilters.length > 0;

      if (!hasIncludeFilter) {
        return statusMatch && methodMatch && pathMatch && clientMatch;
      }

      const includeMatch =
        (statusFilter.length > 0 && statusMatch) ||
        (methodFilter.length > 0 && methodMatch) ||
        (pathFilters.length > 0 && pathMatch) ||
        (clientFilters.length > 0 && clientMatch);

      return includeMatch;
    }

    return statusMatch && methodMatch && pathMatch && clientMatch;
  });
};
