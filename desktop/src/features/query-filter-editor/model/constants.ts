export { HTTP_METHODS, STATUS_CODES } from "@/shared/lib/http-constants";

export const FILTER_KEYWORDS = ["method", "methods", "status", "url", "client"] as const;
export const LOGICAL_OPERATORS = ["and", "or"] as const;
export const COMPARISON_OPERATORS = ["=", "|=", "|~", "!=", "!~"] as const;
