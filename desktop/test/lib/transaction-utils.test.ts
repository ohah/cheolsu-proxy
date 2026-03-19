import { describe, test, expect } from "bun:test";

import {
  uint8ArrayToString,
  uint8ArrayToBase64,
  createImageDataUrl,
  formatBodyContent,
  getBodyForDisplay,
  getFileNameFromUrl,
  getFileNameFromContentDisposition,
  getExtensionFromPath,
  extractBinaryFileInfo,
} from "../../src/features/transaction-details/lib/utils";

// --- uint8ArrayToString ---
describe("uint8ArrayToString", () => {
  test("빈 배열은 빈 문자열 반환", () => {
    expect(uint8ArrayToString(new Uint8Array(), "Text")).toBe("");
  });

  test("null/undefined 입력 시 빈 문자열 반환", () => {
    expect(uint8ArrayToString(null as any, "Text")).toBe("");
  });

  test("UTF-8 문자열 디코딩", () => {
    const encoder = new TextEncoder();
    const data = encoder.encode("Hello, 세계!");
    expect(uint8ArrayToString(data, "Text")).toBe("Hello, 세계!");
  });

  test("일반 number 배열도 처리", () => {
    const data = [72, 101, 108, 108, 111]; // "Hello"
    expect(uint8ArrayToString(data as any, "Text")).toBe("Hello");
  });
});

// --- uint8ArrayToBase64 ---
describe("uint8ArrayToBase64", () => {
  test("빈 배열은 빈 문자열 반환", () => {
    expect(uint8ArrayToBase64(new Uint8Array())).toBe("");
  });

  test("null 입력 시 빈 문자열 반환", () => {
    expect(uint8ArrayToBase64(null as any)).toBe("");
  });

  test("Base64 인코딩 정확성", () => {
    const data = new Uint8Array([72, 101, 108, 108, 111]); // "Hello"
    expect(uint8ArrayToBase64(data)).toBe("SGVsbG8=");
  });

  test("일반 number 배열도 처리", () => {
    const data = [72, 101, 108, 108, 111];
    expect(uint8ArrayToBase64(data)).toBe("SGVsbG8=");
  });
});

// --- createImageDataUrl ---
describe("createImageDataUrl", () => {
  test("Image가 아닌 타입은 빈 문자열 반환", () => {
    expect(createImageDataUrl(new Uint8Array([1, 2, 3]), "Text")).toBe("");
  });

  test("빈 데이터는 빈 문자열 반환", () => {
    expect(createImageDataUrl(new Uint8Array(), "Image")).toBe("");
  });

  test("PNG 시그니처 감지", () => {
    const png = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    const result = createImageDataUrl(png, "Image");
    expect(result).toContain("data:image/png;base64,");
  });

  test("JPEG 시그니처 감지", () => {
    const jpeg = new Uint8Array([0xff, 0xd8, 0xff, 0xe0]);
    const result = createImageDataUrl(jpeg, "Image");
    expect(result).toContain("data:image/jpeg;base64,");
  });

  test("GIF87a 시그니처 감지", () => {
    const gif = new Uint8Array([0x47, 0x49, 0x46, 0x38, 0x37, 0x61]);
    const result = createImageDataUrl(gif, "Image");
    expect(result).toContain("data:image/gif;base64,");
  });

  test("GIF89a 시그니처 감지", () => {
    const gif = new Uint8Array([0x47, 0x49, 0x46, 0x38, 0x39, 0x61]);
    const result = createImageDataUrl(gif, "Image");
    expect(result).toContain("data:image/gif;base64,");
  });

  test("WebP 시그니처 감지", () => {
    const webp = new Uint8Array([
      0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50,
    ]);
    const result = createImageDataUrl(webp, "Image");
    expect(result).toContain("data:image/webp;base64,");
  });

  test("SVG 시그니처 감지", () => {
    const svg = new Uint8Array([0x3c, 0x73, 0x76, 0x67]);
    const result = createImageDataUrl(svg, "Image");
    expect(result).toContain("data:image/svg+xml;base64,");
  });

  test("알 수 없는 이미지는 기본 PNG", () => {
    const unknown = new Uint8Array([0x00, 0x01, 0x02]);
    const result = createImageDataUrl(unknown, "Image");
    expect(result).toContain("data:image/png;base64,");
  });
});

// --- formatBodyContent ---
describe("formatBodyContent", () => {
  test("Empty 타입은 빈 문자열 반환", () => {
    expect(formatBodyContent(new Uint8Array(), "Empty")).toBe("");
  });

  test("JSON bodyJson이 있으면 포맷팅", () => {
    const result = formatBodyContent(new Uint8Array(), "Json", { key: "value" });
    expect(result).toBe('{\n  "key": "value"\n}');
  });

  test("GraphQL bodyJson 포맷팅", () => {
    const graphqlBody = {
      operationName: "GetUsers",
      query: "query GetUsers { users { id name } }",
      variables: { limit: 10 },
    };
    const result = formatBodyContent(new Uint8Array(), "GraphQL", graphqlBody);
    expect(result).toContain("# Operation: GetUsers");
    expect(result).toContain("query GetUsers");
    expect(result).toContain("# Variables");
  });

  test("바이너리 데이터는 크기 표시", () => {
    const data = new Uint8Array(1024);
    const result = formatBodyContent(data, "Binary");
    expect(result).toBe("[Binary - 1024 bytes]");
  });
});

// --- getBodyForDisplay ---
describe("getBodyForDisplay", () => {
  test("Empty 타입은 빈 문자열", () => {
    expect(getBodyForDisplay(new Uint8Array(), "Empty")).toBe("");
  });

  test("바이너리 타입은 안내 메시지 표시", () => {
    const result = getBodyForDisplay(new Uint8Array(100), "Binary");
    expect(result).toContain("바이너리 형식");
    expect(result).toContain("100 bytes");
  });
});

// --- getFileNameFromUrl ---
describe("getFileNameFromUrl", () => {
  test("URL 경로에서 파일명 추출", () => {
    expect(getFileNameFromUrl("http://example.com/path/to/file.png")).toBe("file.png");
  });

  test("확장자 없는 경로는 null", () => {
    expect(getFileNameFromUrl("http://example.com/api/users")).toBeNull();
  });

  test("잘못된 URL은 null", () => {
    expect(getFileNameFromUrl("not-a-url")).toBeNull();
  });

  test("인코딩된 파일명 디코딩", () => {
    expect(getFileNameFromUrl("http://example.com/path/%ED%8C%8C%EC%9D%BC.txt")).toBe("파일.txt");
  });
});

// --- getFileNameFromContentDisposition ---
describe("getFileNameFromContentDisposition", () => {
  test("Content-Disposition 없으면 null", () => {
    expect(getFileNameFromContentDisposition({})).toBeNull();
  });

  test("quoted filename 추출", () => {
    const headers = { "content-disposition": 'attachment; filename="report.pdf"' };
    expect(getFileNameFromContentDisposition(headers)).toBe("report.pdf");
  });

  test("unquoted filename 추출", () => {
    const headers = { "content-disposition": "attachment; filename=data.csv" };
    expect(getFileNameFromContentDisposition(headers)).toBe("data.csv");
  });

  test("UTF-8 filename* 추출", () => {
    const headers = {
      "content-disposition": "attachment; filename*=UTF-8''%ED%8C%8C%EC%9D%BC.pdf",
    };
    expect(getFileNameFromContentDisposition(headers)).toBe("파일.pdf");
  });

  test("filename*가 filename보다 우선", () => {
    const headers = {
      "content-disposition": "attachment; filename=\"fallback.pdf\"; filename*=UTF-8''real.pdf",
    };
    expect(getFileNameFromContentDisposition(headers)).toBe("real.pdf");
  });
});

// --- getExtensionFromPath ---
describe("getExtensionFromPath", () => {
  test("확장자 추출", () => {
    expect(getExtensionFromPath("/path/to/file.png")).toBe("png");
  });

  test("대문자 확장자를 소문자로", () => {
    expect(getExtensionFromPath("/path/to/FILE.PDF")).toBe("pdf");
  });

  test("확장자 없으면 빈 문자열", () => {
    expect(getExtensionFromPath("/path/to/file")).toBe("");
  });

  test("dotfile은 확장자 없음으로 처리 (dot이 첫 문자)", () => {
    // getExtensionFromPath는 dotIndex > 0 조건이므로 .gitignore처럼 첫 문자가 dot이면 빈 문자열
    expect(getExtensionFromPath("/path/.gitignore")).toBe("");
  });
});

// --- extractBinaryFileInfo ---
describe("extractBinaryFileInfo", () => {
  test("Content-Disposition에서 파일명 가져오기", () => {
    const result = extractBinaryFileInfo(
      "http://example.com/download",
      {
        "content-disposition": 'attachment; filename="report.pdf"',
        "content-type": "application/pdf",
      },
      "Document",
    );
    expect(result.fileName).toBe("report.pdf");
    expect(result.fileExtension).toBe("pdf");
    expect(result.mimeType).toBe("application/pdf");
  });

  test("URL에서 파일명 가져오기", () => {
    const result = extractBinaryFileInfo(
      "http://example.com/files/image.png",
      { "content-type": "image/png" },
      "Image",
    );
    expect(result.fileName).toBe("image.png");
  });

  test("파일명 없으면 기본 파일명 생성", () => {
    const result = extractBinaryFileInfo(
      "http://example.com/api/data",
      { "content-type": "application/zip" },
      "Archive",
      undefined,
      2048,
    );
    expect(result.fileName).toBe("file_2048.zip");
    expect(result.fileExtension).toBe("zip");
  });

  test("filePath에서 확장자 추출", () => {
    const result = extractBinaryFileInfo(
      "http://example.com/api/data",
      {},
      "Binary",
      "/cache/body/file.wasm",
    );
    expect(result.fileExtension).toBe("wasm");
  });

  test("MIME 타입에서 확장자 매핑", () => {
    const result = extractBinaryFileInfo(
      "http://example.com/font",
      { "content-type": "font/woff2" },
      "Binary",
    );
    expect(result.fileExtension).toBe("woff2");
  });

  test("DataType 기반 확장자 폴백", () => {
    const result = extractBinaryFileInfo("http://example.com/api", {}, "Document");
    expect(result.fileExtension).toBe("pdf");
  });
});
