import { describe, test, expect } from "bun:test";

import {
  dataTypeToMonacoLanguage,
  dataTypeToMimeType,
  dataTypeToDisplayName,
  isTextBasedDataType,
  isBinaryDataType,
  isMediaDataType,
  isProtobufDataType,
  type DataType,
} from "../../src/shared/lib/data-type";

describe("dataTypeToMonacoLanguage", () => {
  test("Json → json", () => {
    expect(dataTypeToMonacoLanguage("Json")).toBe("json");
  });

  test("Xml → xml", () => {
    expect(dataTypeToMonacoLanguage("Xml")).toBe("xml");
  });

  test("Html → html", () => {
    expect(dataTypeToMonacoLanguage("Html")).toBe("html");
  });

  test("Css → css", () => {
    expect(dataTypeToMonacoLanguage("Css")).toBe("css");
  });

  test("Javascript → javascript", () => {
    expect(dataTypeToMonacoLanguage("Javascript")).toBe("javascript");
  });

  test("GraphQL → graphql", () => {
    expect(dataTypeToMonacoLanguage("GraphQL")).toBe("graphql");
  });

  test("바이너리 타입은 plaintext", () => {
    const binaryTypes: DataType[] = ["Image", "Video", "Audio", "Binary", "Empty", "Unknown"];
    for (const dt of binaryTypes) {
      expect(dataTypeToMonacoLanguage(dt)).toBe("plaintext");
    }
  });
});

describe("dataTypeToMimeType", () => {
  test("Json → application/json", () => {
    expect(dataTypeToMimeType("Json")).toBe("application/json");
  });

  test("Html → text/html", () => {
    expect(dataTypeToMimeType("Html")).toBe("text/html");
  });

  test("Image → image/*", () => {
    expect(dataTypeToMimeType("Image")).toBe("image/*");
  });

  test("Protobuf → application/x-protobuf", () => {
    expect(dataTypeToMimeType("Protobuf")).toBe("application/x-protobuf");
  });

  test("Grpc → application/grpc", () => {
    expect(dataTypeToMimeType("Grpc")).toBe("application/grpc");
  });

  test("Empty → empty", () => {
    expect(dataTypeToMimeType("Empty")).toBe("empty");
  });

  test("Unknown → application/octet-stream", () => {
    expect(dataTypeToMimeType("Unknown")).toBe("application/octet-stream");
  });
});

describe("dataTypeToDisplayName", () => {
  test("주요 타입 표시 이름", () => {
    expect(dataTypeToDisplayName("Json")).toBe("JSON");
    expect(dataTypeToDisplayName("Html")).toBe("HTML");
    expect(dataTypeToDisplayName("Grpc")).toBe("gRPC");
    expect(dataTypeToDisplayName("Binary")).toBe("Binary Data");
    expect(dataTypeToDisplayName("Empty")).toBe("Empty");
    expect(dataTypeToDisplayName("Unknown")).toBe("Unknown");
  });
});

describe("isTextBasedDataType", () => {
  test("텍스트 기반 타입 true", () => {
    const textTypes: DataType[] = ["Json", "Xml", "Html", "Css", "Javascript", "GraphQL", "Text"];
    for (const dt of textTypes) {
      expect(isTextBasedDataType(dt)).toBe(true);
    }
  });

  test("바이너리 타입 false", () => {
    const binaryTypes: DataType[] = ["Image", "Video", "Audio", "Binary", "Protobuf"];
    for (const dt of binaryTypes) {
      expect(isTextBasedDataType(dt)).toBe(false);
    }
  });
});

describe("isBinaryDataType", () => {
  test("바이너리 타입 true", () => {
    const binaryTypes: DataType[] = [
      "Image",
      "Video",
      "Audio",
      "Document",
      "Archive",
      "Protobuf",
      "Grpc",
      "Binary",
    ];
    for (const dt of binaryTypes) {
      expect(isBinaryDataType(dt)).toBe(true);
    }
  });

  test("텍스트 타입 false", () => {
    const textTypes: DataType[] = ["Json", "Text", "Html", "Empty"];
    for (const dt of textTypes) {
      expect(isBinaryDataType(dt)).toBe(false);
    }
  });
});

describe("isMediaDataType", () => {
  test("미디어 타입 true", () => {
    expect(isMediaDataType("Image")).toBe(true);
    expect(isMediaDataType("Video")).toBe(true);
    expect(isMediaDataType("Audio")).toBe(true);
  });

  test("비미디어 타입 false", () => {
    expect(isMediaDataType("Document")).toBe(false);
    expect(isMediaDataType("Binary")).toBe(false);
    expect(isMediaDataType("Json")).toBe(false);
  });
});

describe("isProtobufDataType", () => {
  test("Protobuf와 Grpc true", () => {
    expect(isProtobufDataType("Protobuf")).toBe(true);
    expect(isProtobufDataType("Grpc")).toBe(true);
  });

  test("다른 타입 false", () => {
    expect(isProtobufDataType("Binary")).toBe(false);
    expect(isProtobufDataType("Json")).toBe(false);
  });
});
