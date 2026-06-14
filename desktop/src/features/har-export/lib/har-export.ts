import type { HttpTransaction } from "@/entities/proxy";
import { dataTypeToMimeType } from "@/entities/proxy";
import { readFile, BaseDirectory } from "@tauri-apps/plugin-fs";

interface HarLog {
  log: {
    version: string;
    creator: { name: string; version: string };
    entries: HarEntry[];
  };
}

interface HarEntry {
  startedDateTime: string;
  time: number;
  request: HarRequest;
  response: HarResponse;
  cache: Record<string, never>;
  timings: { send: number; wait: number; receive: number };
}

interface HarRequest {
  method: string;
  url: string;
  httpVersion: string;
  cookies: HarCookie[];
  headers: HarHeader[];
  queryString: HarQueryParam[];
  postData?: { mimeType: string; text: string };
  headersSize: number;
  bodySize: number;
}

interface HarResponse {
  status: number;
  statusText: string;
  httpVersion: string;
  cookies: HarCookie[];
  headers: HarHeader[];
  content: {
    size: number;
    mimeType: string;
    text?: string;
    encoding?: string;
  };
  redirectURL: string;
  headersSize: number;
  bodySize: number;
}

interface HarHeader {
  name: string;
  value: string;
}

interface HarCookie {
  name: string;
  value: string;
}

interface HarQueryParam {
  name: string;
  value: string;
}

function headersToHar(headers: Record<string, string>): HarHeader[] {
  return Object.entries(headers).map(([name, value]) => ({ name, value }));
}

function parseCookies(headers: Record<string, string>, headerName: string): HarCookie[] {
  const cookieHeader = headers[headerName];
  if (!cookieHeader) return [];

  return cookieHeader.split(";").map((pair) => {
    const [name, ...rest] = pair.trim().split("=");
    return { name: name?.trim() ?? "", value: rest.join("=")?.trim() ?? "" };
  });
}

function parseQueryString(uri: string): HarQueryParam[] {
  try {
    const url = new URL(uri);
    const params: HarQueryParam[] = [];
    url.searchParams.forEach((value, name) => {
      params.push({ name, value });
    });
    return params;
  } catch {
    return [];
  }
}

function versionToString(version: string): string {
  if (version === "HTTP/2.0" || version === "h2") return "HTTP/2.0";
  if (version === "HTTP/1.0") return "HTTP/1.0";
  return "HTTP/1.1";
}

function computeHeadersSize(headers: Record<string, string>): number {
  let size = 0;
  for (const [name, value] of Object.entries(headers)) {
    size += name.length + 2 + value.length + 2; // "name: value\r\n"
  }
  return size > 0 ? size + 2 : -1; // +2 for final \r\n
}

function statusText(status: number): string {
  const map: Record<number, string> = {
    200: "OK",
    201: "Created",
    204: "No Content",
    301: "Moved Permanently",
    302: "Found",
    304: "Not Modified",
    400: "Bad Request",
    401: "Unauthorized",
    403: "Forbidden",
    404: "Not Found",
    405: "Method Not Allowed",
    500: "Internal Server Error",
    502: "Bad Gateway",
    503: "Service Unavailable",
    504: "Gateway Timeout",
  };
  return map[status] ?? "";
}

async function readBodyContent(
  body: Uint8Array | null,
  filePath: string | undefined,
): Promise<Uint8Array | null> {
  if (body && body.length > 0) return body;
  if (!filePath) return null;

  try {
    const appCachePath = filePath.startsWith("com.cheolsu-proxy/")
      ? filePath.slice("com.cheolsu-proxy/".length)
      : filePath;
    return await readFile(appCachePath, { baseDir: BaseDirectory.AppCache });
  } catch {
    return null;
  }
}

function encodeBody(data: Uint8Array): { text: string; encoding?: string } {
  try {
    const decoder = new TextDecoder("utf-8", { fatal: true });
    return { text: decoder.decode(data) };
  } catch {
    let binary = "";
    for (let i = 0; i < data.length; i++) {
      binary += String.fromCharCode(data[i]);
    }
    return { text: btoa(binary), encoding: "base64" };
  }
}

export async function buildHarLog(transactions: HttpTransaction[]): Promise<HarLog> {
  const entries: HarEntry[] = [];

  for (const tx of transactions) {
    const { request, response } = tx;
    if (!request) continue;

    const reqHeaders = request.headers ?? {};
    const reqBody = await readBodyContent(request.body, request.file_path);

    const harRequest: HarRequest = {
      method: request.method,
      url: request.uri,
      httpVersion: versionToString(request.version),
      cookies: parseCookies(reqHeaders, "cookie"),
      headers: headersToHar(reqHeaders),
      queryString: parseQueryString(request.uri),
      headersSize: computeHeadersSize(reqHeaders),
      bodySize: request.body_size,
    };

    if (reqBody && reqBody.length > 0) {
      const mimeType = reqHeaders["content-type"] ?? dataTypeToMimeType(request.data_type);
      const { text } = encodeBody(reqBody);
      harRequest.postData = { mimeType, text };
    }

    let harResponse: HarResponse;
    if (response) {
      const resHeaders = response.headers ?? {};
      const resBody = await readBodyContent(response.body, response.file_path);

      const content: HarResponse["content"] = {
        size: response.body_size,
        mimeType: resHeaders["content-type"] ?? dataTypeToMimeType(response.data_type),
      };

      if (resBody && resBody.length > 0) {
        const encoded = encodeBody(resBody);
        content.text = encoded.text;
        if (encoded.encoding) content.encoding = encoded.encoding;
      }

      harResponse = {
        status: response.status,
        statusText: statusText(response.status),
        httpVersion: versionToString(response.version),
        cookies: parseCookies(resHeaders, "set-cookie"),
        headers: headersToHar(resHeaders),
        content,
        redirectURL: resHeaders["location"] ?? "",
        headersSize: computeHeadersSize(resHeaders),
        bodySize: response.body_size,
      };
    } else {
      harResponse = {
        status: 0,
        statusText: "",
        httpVersion: "HTTP/1.1",
        cookies: [],
        headers: [],
        content: { size: 0, mimeType: "x-unknown" },
        redirectURL: "",
        headersSize: -1,
        bodySize: -1,
      };
    }

    const requestTime = request.time;
    const responseTime = response?.time ?? requestTime;
    const elapsed = Math.max(0, responseTime - requestTime);

    entries.push({
      startedDateTime: new Date(requestTime).toISOString(),
      time: elapsed,
      request: harRequest,
      response: harResponse,
      cache: {},
      timings: {
        send: 0,
        wait: elapsed,
        receive: 0,
      },
    });
  }

  return {
    log: {
      version: "1.2",
      creator: { name: "Cheolsu Proxy", version: "0.1.2" },
      entries,
    },
  };
}
