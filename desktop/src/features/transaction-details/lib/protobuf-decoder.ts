/** Protobuf wire type */
export enum WireType {
  Varint = 0,
  Fixed64 = 1,
  LengthDelimited = 2,
  StartGroup = 3,
  EndGroup = 4,
  Fixed32 = 5,
}

export const wireTypeName = (wt: WireType): string => {
  switch (wt) {
    case WireType.Varint:
      return "varint";
    case WireType.Fixed64:
      return "fixed64";
    case WireType.LengthDelimited:
      return "len";
    case WireType.StartGroup:
      return "sgroup";
    case WireType.EndGroup:
      return "egroup";
    case WireType.Fixed32:
      return "fixed32";
    default:
      return "unknown";
  }
};

export type ProtobufValue =
  | { type: "varint"; raw: bigint; signed: bigint; bool: boolean }
  | { type: "fixed64"; raw: bigint; asDouble: number }
  | { type: "fixed32"; raw: number; asFloat: number }
  | { type: "message"; fields: ProtobufField[] }
  | { type: "string"; value: string }
  | { type: "bytes"; value: Uint8Array };

export interface ProtobufField {
  fieldNumber: number;
  wireType: WireType;
  value: ProtobufValue;
}

/** varint 읽기 (최대 10바이트) */
function readVarint(data: Uint8Array, offset: number): [bigint, number] {
  let result = 0n;
  let shift = 0n;
  let pos = offset;

  for (let i = 0; i < 10; i++) {
    if (pos >= data.length) throw new Error("unexpected end of varint");
    const byte = data[pos];
    pos += 1;
    result |= BigInt(byte & 0x7f) << shift;
    if ((byte & 0x80) === 0) return [result, pos];
    shift += 7n;
  }
  throw new Error("varint too long");
}

/** ZigZag 디코딩 (sint32/sint64) */
function decodeZigZag(value: bigint): bigint {
  return (value >> 1n) ^ -(value & 1n);
}

/** 64-bit little-endian 읽기 */
function readFixed64(data: Uint8Array, offset: number): bigint {
  if (offset + 8 > data.length) throw new Error("unexpected end of fixed64");
  const view = new DataView(data.buffer, data.byteOffset + offset, 8);
  return view.getBigUint64(0, true);
}

/** 32-bit little-endian 읽기 */
function readFixed32(data: Uint8Array, offset: number): number {
  if (offset + 4 > data.length) throw new Error("unexpected end of fixed32");
  const view = new DataView(data.buffer, data.byteOffset + offset, 4);
  return view.getUint32(0, true);
}

/** fixed64를 double로 변환 */
function fixed64ToDouble(value: bigint): number {
  const buf = new ArrayBuffer(8);
  new DataView(buf).setBigUint64(0, value, true);
  return new DataView(buf).getFloat64(0, true);
}

/** fixed32를 float로 변환 */
function fixed32ToFloat(value: number): number {
  const buf = new ArrayBuffer(4);
  new DataView(buf).setUint32(0, value, true);
  return new DataView(buf).getFloat32(0, true);
}

/** 유효한 UTF-8 프린터블 문자열인지 확인 */
function isLikelyString(data: Uint8Array): boolean {
  if (data.length === 0) return false;
  try {
    const str = new TextDecoder("utf-8", { fatal: true }).decode(data);
    // 제어문자(탭/개행/CR 제외)가 포함되면 문자열이 아닌 것으로 판단
    for (let i = 0; i < str.length; i++) {
      const code = str.charCodeAt(i);
      if (code < 0x20 && code !== 0x09 && code !== 0x0a && code !== 0x0d) {
        return false;
      }
    }
    return true;
  } catch {
    return false;
  }
}

/** length-delimited 데이터를 중첩 메시지로 파싱 시도 */
function tryDecodeAsMessage(data: Uint8Array): ProtobufField[] | null {
  // 2바이트 미만은 중첩 메시지로 판별하지 않음 (오탐 방지)
  if (data.length < 2) return null;
  try {
    const fields = parseFields(data);
    // 파싱 결과 검증: 필드가 하나 이상이고, field number가 합리적인 범위 내
    if (fields.length === 0) return null;
    for (const f of fields) {
      if (f.fieldNumber <= 0 || f.fieldNumber > 536870911) return null; // max field number: 2^29-1
      if (f.wireType > 5) return null;
    }
    return fields;
  } catch {
    return null;
  }
}

/** wire format 필드 파싱 (내부 재귀용) */
function parseFields(data: Uint8Array): ProtobufField[] {
  const fields: ProtobufField[] = [];
  let offset = 0;

  while (offset < data.length) {
    const [tag, newOffset] = readVarint(data, offset);
    offset = newOffset;

    const fieldNumber = Number(tag >> 3n);
    const wireType = Number(tag & 7n) as WireType;

    if (fieldNumber <= 0) throw new Error("invalid field number");

    let value: ProtobufValue;

    switch (wireType) {
      case WireType.Varint: {
        const [raw, nextOffset] = readVarint(data, offset);
        offset = nextOffset;
        value = {
          type: "varint",
          raw,
          signed: decodeZigZag(raw),
          bool: raw !== 0n,
        };
        break;
      }
      case WireType.Fixed64: {
        const raw = readFixed64(data, offset);
        offset += 8;
        value = { type: "fixed64", raw, asDouble: fixed64ToDouble(raw) };
        break;
      }
      case WireType.Fixed32: {
        const raw = readFixed32(data, offset);
        offset += 4;
        value = { type: "fixed32", raw, asFloat: fixed32ToFloat(raw) };
        break;
      }
      case WireType.LengthDelimited: {
        const [len, lenOffset] = readVarint(data, offset);
        offset = lenOffset;
        const length = Number(len);
        if (offset + length > data.length) throw new Error("length-delimited overflow");
        const payload = data.subarray(offset, offset + length);
        offset += length;

        // 휴리스틱: 중첩 메시지 → 문자열 → raw bytes
        const nested = tryDecodeAsMessage(payload);
        if (nested) {
          value = { type: "message", fields: nested };
        } else if (isLikelyString(payload)) {
          value = {
            type: "string",
            value: new TextDecoder().decode(payload),
          };
        } else {
          value = { type: "bytes", value: payload };
        }
        break;
      }
      case WireType.StartGroup:
      case WireType.EndGroup:
        // deprecated, skip
        throw new Error("group wire types not supported");
      default:
        throw new Error(`unknown wire type: ${wireType}`);
    }

    fields.push({ fieldNumber, wireType, value });
  }

  return fields;
}

/** gRPC 프레이밍 헤더 제거 (5바이트: 1B compressed + 4B BE length) */
export function stripGrpcFraming(data: Uint8Array): Uint8Array[] {
  const messages: Uint8Array[] = [];
  let offset = 0;

  while (offset + 5 <= data.length) {
    const compressedFlag = data[offset];
    if (compressedFlag !== 0) {
      // 압축된 gRPC 메시지는 현재 미지원
      break;
    }
    offset += 1;
    const view = new DataView(data.buffer, data.byteOffset + offset, 4);
    const msgLen = view.getUint32(0, false); // big-endian
    offset += 4;

    if (offset + msgLen > data.length) break;
    messages.push(data.subarray(offset, offset + msgLen));
    offset += msgLen;
  }

  return messages.length > 0 ? messages : [data];
}

/** Content-Type이 gRPC인지 확인 */
export function isGrpcContentType(contentType: string): boolean {
  const ct = contentType.toLowerCase();
  return ct.includes("grpc");
}

/** 메인 디코딩 함수 */
export function decodeProtobuf(
  data: Uint8Array,
  contentType?: string,
): { fields: ProtobufField[]; isGrpc: boolean; messageCount: number } {
  const isGrpc = contentType ? isGrpcContentType(contentType) : false;

  if (isGrpc) {
    const messages = stripGrpcFraming(data);
    // 첫 번째 메시지만 디코딩 (여러 메시지 시 추후 확장)
    const fields = parseFields(messages[0]);
    return { fields, isGrpc, messageCount: messages.length };
  }

  const fields = parseFields(data);
  return { fields, isGrpc, messageCount: 1 };
}

/** 바이트 배열을 hex 문자열로 변환 */
export function bytesToHex(data: Uint8Array, maxBytes = 32): string {
  const hex = Array.from(data.subarray(0, maxBytes))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join(" ");
  return data.length > maxBytes ? `${hex} ...` : hex;
}
