import { describe, test, expect } from "bun:test";

import {
  decodeProtobuf,
  stripGrpcFraming,
  isGrpcContentType,
  bytesToHex,
  WireType,
} from "../../src/features/transaction-details/lib/protobuf-decoder";

/** protobuf 바이너리를 직접 생성하는 헬퍼 */
function encodeVarint(value: number): number[] {
  const bytes: number[] = [];
  let v = value >>> 0; // unsigned
  while (v > 0x7f) {
    bytes.push((v & 0x7f) | 0x80);
    v >>>= 7;
  }
  bytes.push(v);
  return bytes;
}

function encodeTag(fieldNumber: number, wireType: number): number[] {
  return encodeVarint((fieldNumber << 3) | wireType);
}

function encodeString(fieldNumber: number, str: string): number[] {
  const encoded = new TextEncoder().encode(str);
  return [...encodeTag(fieldNumber, 2), ...encodeVarint(encoded.length), ...encoded];
}

function encodeVarintField(fieldNumber: number, value: number): number[] {
  return [...encodeTag(fieldNumber, 0), ...encodeVarint(value)];
}

describe("protobuf-decoder", () => {
  describe("varint 디코딩", () => {
    test("단일 바이트 varint", () => {
      // field 1, varint, value = 1
      const data = new Uint8Array([0x08, 0x01]);
      const result = decodeProtobuf(data);
      expect(result.fields).toHaveLength(1);
      expect(result.fields[0].fieldNumber).toBe(1);
      expect(result.fields[0].wireType).toBe(WireType.Varint);
      expect(result.fields[0].value.type).toBe("varint");
      if (result.fields[0].value.type === "varint") {
        expect(result.fields[0].value.raw).toBe(1n);
        expect(result.fields[0].value.bool).toBe(true);
      }
    });

    test("멀티 바이트 varint (150)", () => {
      // field 1, varint, value = 150 (0x96 0x01)
      const data = new Uint8Array([0x08, 0x96, 0x01]);
      const result = decodeProtobuf(data);
      expect(result.fields).toHaveLength(1);
      if (result.fields[0].value.type === "varint") {
        expect(result.fields[0].value.raw).toBe(150n);
      }
    });

    test("zigzag 디코딩 (signed)", () => {
      // field 1, varint, value = 1 → zigzag signed = -1
      const data = new Uint8Array([0x08, 0x01]);
      const result = decodeProtobuf(data);
      if (result.fields[0].value.type === "varint") {
        expect(result.fields[0].value.signed).toBe(-1n);
      }
    });

    test("bool 해석 (value=0 → false)", () => {
      const data = new Uint8Array([0x08, 0x00]);
      const result = decodeProtobuf(data);
      if (result.fields[0].value.type === "varint") {
        expect(result.fields[0].value.bool).toBe(false);
      }
    });
  });

  describe("fixed32 디코딩", () => {
    test("fixed32 정수 값", () => {
      // field 1, fixed32 (wire type 5), value = 100 (little-endian)
      const data = new Uint8Array([0x0d, 0x64, 0x00, 0x00, 0x00]);
      const result = decodeProtobuf(data);
      expect(result.fields).toHaveLength(1);
      expect(result.fields[0].wireType).toBe(WireType.Fixed32);
      if (result.fields[0].value.type === "fixed32") {
        expect(result.fields[0].value.raw).toBe(100);
      }
    });

    test("fixed32 float 해석", () => {
      // float 3.14 = 0x4048F5C3 (little-endian: C3 F5 48 40)
      const data = new Uint8Array([0x0d, 0xc3, 0xf5, 0x48, 0x40]);
      const result = decodeProtobuf(data);
      if (result.fields[0].value.type === "fixed32") {
        expect(result.fields[0].value.asFloat).toBeCloseTo(3.14, 2);
      }
    });
  });

  describe("fixed64 디코딩", () => {
    test("fixed64 정수 값", () => {
      // field 1, fixed64 (wire type 1), value = 1 (little-endian)
      const data = new Uint8Array([0x09, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
      const result = decodeProtobuf(data);
      expect(result.fields).toHaveLength(1);
      expect(result.fields[0].wireType).toBe(WireType.Fixed64);
      if (result.fields[0].value.type === "fixed64") {
        expect(result.fields[0].value.raw).toBe(1n);
      }
    });
  });

  describe("length-delimited 디코딩", () => {
    test("문자열 필드", () => {
      const data = new Uint8Array(encodeString(1, "hello"));
      const result = decodeProtobuf(data);
      expect(result.fields).toHaveLength(1);
      expect(result.fields[0].value.type).toBe("string");
      if (result.fields[0].value.type === "string") {
        expect(result.fields[0].value.value).toBe("hello");
      }
    });

    test("중첩 메시지", () => {
      // 내부 메시지: field 1 = 42
      const inner = encodeVarintField(1, 42);
      // 외부 메시지: field 2 = 내부 메시지 (length-delimited)
      const data = new Uint8Array([...encodeTag(2, 2), ...encodeVarint(inner.length), ...inner]);
      const result = decodeProtobuf(data);
      expect(result.fields).toHaveLength(1);
      expect(result.fields[0].value.type).toBe("message");
      if (result.fields[0].value.type === "message") {
        expect(result.fields[0].value.fields).toHaveLength(1);
        expect(result.fields[0].value.fields[0].fieldNumber).toBe(1);
      }
    });

    test("바이너리 데이터는 bytes로 처리", () => {
      // 유효하지 않은 UTF-8이고 유효한 protobuf도 아닌 데이터
      const payload = [0xff, 0xfe, 0xfd, 0xfc, 0xfb];
      const data = new Uint8Array([
        ...encodeTag(1, 2),
        ...encodeVarint(payload.length),
        ...payload,
      ]);
      const result = decodeProtobuf(data);
      expect(result.fields[0].value.type).toBe("bytes");
    });

    test("1바이트 데이터는 중첩 메시지로 오탐하지 않음", () => {
      // 1바이트 페이로드: tryDecodeAsMessage가 최소 2바이트 체크
      const data = new Uint8Array([...encodeTag(1, 2), 0x01, 0x41]); // "A"
      const result = decodeProtobuf(data);
      expect(result.fields[0].value.type).toBe("string");
    });
  });

  describe("복합 메시지", () => {
    test("여러 필드가 있는 메시지", () => {
      const data = new Uint8Array([
        ...encodeVarintField(1, 42),
        ...encodeString(2, "test"),
        ...encodeVarintField(3, 1),
      ]);
      const result = decodeProtobuf(data);
      expect(result.fields).toHaveLength(3);
      expect(result.fields[0].fieldNumber).toBe(1);
      expect(result.fields[1].fieldNumber).toBe(2);
      expect(result.fields[2].fieldNumber).toBe(3);
    });
  });

  describe("gRPC 프레이밍", () => {
    test("gRPC 프레이밍 헤더 제거", () => {
      const protobufData = new Uint8Array([0x08, 0x96, 0x01]); // field 1 = 150
      // gRPC frame: compressed=0, length=3, data
      const grpcFrame = new Uint8Array([
        0x00, // compressed flag
        0x00,
        0x00,
        0x00,
        0x03, // length (big-endian)
        ...protobufData,
      ]);
      const messages = stripGrpcFraming(grpcFrame);
      expect(messages).toHaveLength(1);
      expect(messages[0]).toEqual(protobufData);
    });

    test("압축된 gRPC 메시지는 건너뜀", () => {
      const grpcFrame = new Uint8Array([
        0x01, // compressed flag = 1
        0x00,
        0x00,
        0x00,
        0x03,
        0x08,
        0x96,
        0x01,
      ]);
      // compressed flag가 1이면 break하므로 원본 데이터 반환
      const messages = stripGrpcFraming(grpcFrame);
      expect(messages).toHaveLength(1);
      expect(messages[0]).toEqual(grpcFrame);
    });

    test("gRPC Content-Type 감지", () => {
      const result = decodeProtobuf(
        new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x03, 0x08, 0x96, 0x01]),
        "application/grpc",
      );
      expect(result.isGrpc).toBe(true);
      expect(result.fields).toHaveLength(1);
    });
  });

  describe("isGrpcContentType", () => {
    test("gRPC Content-Type 감지", () => {
      expect(isGrpcContentType("application/grpc")).toBe(true);
      expect(isGrpcContentType("application/grpc+proto")).toBe(true);
      expect(isGrpcContentType("application/grpc-web")).toBe(true);
      expect(isGrpcContentType("application/json")).toBe(false);
    });
  });

  describe("bytesToHex", () => {
    test("바이트를 hex 문자열로 변환", () => {
      const data = new Uint8Array([0x08, 0x96, 0x01]);
      expect(bytesToHex(data, 32)).toBe("08 96 01");
    });

    test("maxBytes 초과 시 ... 추가", () => {
      const data = new Uint8Array([0x01, 0x02, 0x03, 0x04]);
      expect(bytesToHex(data, 2)).toBe("01 02 ...");
    });
  });

  describe("에러 처리", () => {
    test("빈 데이터", () => {
      const data = new Uint8Array([]);
      const result = decodeProtobuf(data);
      expect(result.fields).toHaveLength(0);
    });

    test("잘못된 데이터는 에러 발생", () => {
      // 잘린 varint (continuation bit 설정됐지만 다음 바이트 없음)
      const data = new Uint8Array([0x08, 0x80]);
      expect(() => decodeProtobuf(data)).toThrow();
    });
  });
});
