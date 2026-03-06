import type { WsContentType } from "@/entities/websocket/model/types";

// --- Socket.IO Parser ---

const SOCKETIO_TYPES: Record<string, string> = {
  "0": "CONNECT",
  "1": "DISCONNECT",
  "2": "EVENT",
  "3": "ACK",
  "4": "CONNECT_ERROR",
};

export function formatSocketIO(payload: string): string {
  // Engine.IO packet type is first char (should be '4' for message)
  // Socket.IO packet type is second char
  if (payload.length < 2) return payload;

  const sioType = SOCKETIO_TYPES[payload[1]] ?? `UNKNOWN(${payload[1]})`;
  let rest = payload.slice(2);
  const lines: string[] = [`# Socket.IO ${sioType}`];

  // Parse optional namespace (starts with /)
  if (rest.startsWith("/")) {
    const commaIdx = rest.indexOf(",");
    if (commaIdx !== -1) {
      lines.push(`Namespace: ${rest.slice(0, commaIdx)}`);
      rest = rest.slice(commaIdx + 1);
    } else {
      lines.push(`Namespace: ${rest}`);
      rest = "";
    }
  }

  // Parse optional ack ID (digits before [ or {)
  const ackMatch = rest.match(/^(\d+)([[{].*)$/s);
  if (ackMatch) {
    lines.push(`Ack ID: ${ackMatch[1]}`);
    rest = ackMatch[2];
  }

  if (rest) {
    // Try to parse as JSON array for EVENT type: ["eventName", ...data]
    try {
      const parsed = JSON.parse(rest);
      if (Array.isArray(parsed) && parsed.length > 0 && typeof parsed[0] === "string") {
        lines.push(`Event: ${parsed[0]}`);
        if (parsed.length > 1) {
          const data = parsed.length === 2 ? parsed[1] : parsed.slice(1);
          lines.push(`Data:\n${JSON.stringify(data, null, 2)}`);
        }
      } else {
        lines.push(`Data:\n${JSON.stringify(parsed, null, 2)}`);
      }
    } catch {
      lines.push(`Data:\n${rest}`);
    }
  }

  return lines.join("\n");
}

// --- MQTT Parser ---

const MQTT_PACKET_TYPES: Record<number, string> = {
  1: "CONNECT",
  2: "CONNACK",
  3: "PUBLISH",
  4: "PUBACK",
  5: "PUBREC",
  6: "PUBREL",
  7: "PUBCOMP",
  8: "SUBSCRIBE",
  9: "SUBACK",
  10: "UNSUBSCRIBE",
  11: "UNSUBACK",
  12: "PINGREQ",
  13: "PINGRESP",
  14: "DISCONNECT",
};

function decodeBase64ToBytes(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function decodeMqttRemainingLength(bytes: Uint8Array, offset: number): [number, number] {
  let multiplier = 1;
  let value = 0;
  let idx = offset;
  while (idx < bytes.length) {
    const byte = bytes[idx];
    value += (byte & 0x7f) * multiplier;
    idx += 1;
    if ((byte & 0x80) === 0) break;
    multiplier *= 128;
  }
  return [value, idx];
}

/**
 * MQTT 5 Properties 섹션을 건너뛴다.
 * Properties Length (Variable Byte Integer) + Properties 바이트를 skip.
 */
function skipMqtt5Properties(bytes: Uint8Array, offset: number): number {
  const [propLen, afterLen] = decodeMqttRemainingLength(bytes, offset);
  return afterLen + propLen;
}

/**
 * SUBSCRIBE 토픽 파싱을 시도하고, 모든 QoS가 유효(0-2)한지 검증.
 * 유효하면 파싱 결과를, 아니면 null을 반환.
 */
function tryParseSubscribeTopics(
  bytes: Uint8Array,
  offset: number,
  end: number,
): { label: string; value: string }[] | null {
  const subs: string[] = [];
  let off = offset;
  while (off < end) {
    const [topic, newOff] = decodeMqttUtf8String(bytes, off);
    off = newOff;
    if (off >= bytes.length) return null;
    const qos = bytes[off];
    off += 1;
    if (qos > 2) return null; // QoS는 0, 1, 2만 유효
    subs.push(`${topic} (QoS ${qos})`);
  }
  if (subs.length === 0) return null;
  return [{ label: "Subscriptions", value: subs.join(", ") }];
}

function decodeMqttUtf8String(bytes: Uint8Array, offset: number): [string, number] {
  if (offset + 2 > bytes.length) return ["", offset];
  const len = (bytes[offset] << 8) | bytes[offset + 1];
  const str = new TextDecoder().decode(bytes.slice(offset + 2, offset + 2 + len));
  return [str, offset + 2 + len];
}

export interface MqttMeta {
  packetType: string;
  fields: { label: string; value: string }[];
}

export interface MqttParsed {
  meta: MqttMeta;
  payload: string;
  payloadLanguage: string;
}

export function parseMqtt(base64Payload: string): MqttParsed | null {
  try {
    const bytes = decodeBase64ToBytes(base64Payload);
    if (bytes.length < 2) return null;

    const packetTypeNum = bytes[0] >> 4;
    const flags = bytes[0] & 0x0f;
    const packetType = MQTT_PACKET_TYPES[packetTypeNum] ?? `UNKNOWN(${packetTypeNum})`;
    const [remainingLength, payloadStart] = decodeMqttRemainingLength(bytes, 1);
    const fields: { label: string; value: string }[] = [];
    let payload = "";
    let payloadLanguage = "plaintext";

    if (packetTypeNum === 3) {
      // PUBLISH
      const qos = (flags & 0x06) >> 1;
      const retain = (flags & 0x01) !== 0;
      const dup = (flags & 0x08) !== 0;

      let offset = payloadStart;
      const [topic, newOffset] = decodeMqttUtf8String(bytes, offset);
      offset = newOffset;

      let packetId: number | undefined;
      if (qos > 0 && offset + 2 <= bytes.length) {
        packetId = (bytes[offset] << 8) | bytes[offset + 1];
        offset += 2;
      }

      fields.push({ label: "Topic", value: topic });
      fields.push({ label: "QoS", value: String(qos) });
      fields.push({ label: "Retain", value: String(retain) });
      if (dup) fields.push({ label: "DUP", value: "true" });
      if (packetId !== undefined) fields.push({ label: "Packet ID", value: String(packetId) });

      const payloadBytes = bytes.slice(offset, payloadStart + remainingLength);
      if (payloadBytes.length > 0) {
        const payloadText = new TextDecoder("utf-8", { fatal: false }).decode(payloadBytes);
        try {
          const json = JSON.parse(payloadText);
          payload = JSON.stringify(json, null, 2);
          payloadLanguage = "json";
        } catch {
          payload = payloadText;
        }
      }
    } else if (packetTypeNum === 1) {
      // CONNECT
      let offset = payloadStart;
      const [protocolName, off1] = decodeMqttUtf8String(bytes, offset);
      offset = off1;
      fields.push({ label: "Protocol", value: protocolName });
      if (offset < bytes.length) {
        fields.push({ label: "Version", value: String(bytes[offset]) });
        offset += 1;
      }
      if (offset < bytes.length) {
        const connectFlags = bytes[offset];
        fields.push({ label: "Clean Session", value: String((connectFlags & 0x02) !== 0) });
        offset += 1;
      }
      if (offset + 2 <= bytes.length) {
        const keepAlive = (bytes[offset] << 8) | bytes[offset + 1];
        fields.push({ label: "Keep Alive", value: `${keepAlive}s` });
        offset += 2;
      }
      const [clientId] = decodeMqttUtf8String(bytes, offset);
      if (clientId) fields.push({ label: "Client ID", value: clientId });
    } else if (packetTypeNum === 2) {
      // CONNACK
      if (payloadStart + 1 < bytes.length) {
        fields.push({
          label: "Session Present",
          value: String((bytes[payloadStart] & 0x01) !== 0),
        });
        fields.push({ label: "Reason Code", value: String(bytes[payloadStart + 1]) });
      }
    } else if (packetTypeNum === 8) {
      // SUBSCRIBE
      let offset = payloadStart;
      const end = payloadStart + remainingLength;
      if (offset + 2 <= bytes.length) {
        fields.push({
          label: "Packet ID",
          value: String((bytes[offset] << 8) | bytes[offset + 1]),
        });
        offset += 2;
      }
      // MQTT 5: Properties 섹션이 있을 수 있음. 먼저 Properties 건너뛴 후 시도, 실패하면 직접 파싱
      const mqtt5Offset = skipMqtt5Properties(bytes, offset);
      const mqtt5Result = tryParseSubscribeTopics(bytes, mqtt5Offset, end);
      if (mqtt5Result) {
        fields.push(...mqtt5Result);
      } else {
        // MQTT 3.1.1: Properties 없이 바로 토픽 파싱
        const mqtt3Result = tryParseSubscribeTopics(bytes, offset, end);
        if (mqtt3Result) {
          fields.push(...mqtt3Result);
        }
      }
    } else if (packetTypeNum === 9) {
      // SUBACK
      let offset = payloadStart;
      const end = payloadStart + remainingLength;
      if (offset + 2 <= bytes.length) {
        fields.push({
          label: "Packet ID",
          value: String((bytes[offset] << 8) | bytes[offset + 1]),
        });
        offset += 2;
      }
      // MQTT 5: Properties 건너뛰기 시도
      const mqtt5Offset = skipMqtt5Properties(bytes, offset);
      const useOffset = mqtt5Offset <= end && mqtt5Offset > offset ? mqtt5Offset : offset;
      const codes: number[] = [];
      let codeOff = useOffset;
      while (codeOff < end) {
        codes.push(bytes[codeOff]);
        codeOff += 1;
      }
      // MQTT 5 결과가 이상하면 (코드가 모두 > 2일 때) MQTT 3.1.1로 재시도
      if (codes.length === 0 || codes.every((c) => c > 2)) {
        codes.length = 0;
        let fallbackOff = offset;
        while (fallbackOff < end) {
          codes.push(bytes[fallbackOff]);
          fallbackOff += 1;
        }
      }
      if (codes.length > 0) fields.push({ label: "Return Codes", value: codes.join(", ") });
    }

    return { meta: { packetType, fields }, payload, payloadLanguage };
  } catch {
    return null;
  }
}

/**
 * MQTT 패킷 타입명을 반환 (테이블 표시용)
 */
export function getMqttPacketType(base64Payload: string): string | null {
  try {
    const bytes = decodeBase64ToBytes(base64Payload);
    if (bytes.length < 2) return null;
    const packetTypeNum = bytes[0] >> 4;
    return MQTT_PACKET_TYPES[packetTypeNum] ?? null;
  } catch {
    return null;
  }
}

/**
 * WebSocket content type에 따라 Monaco 언어와 포맷된 텍스트를 반환
 */
export function getWsContentView(
  payload: string,
  isBinary: boolean,
  contentType?: WsContentType,
): { language: string; formatted: string } {
  if (contentType === "socket_io") {
    return { language: "socketio", formatted: formatSocketIO(payload) };
  }

  if (contentType === "mqtt") {
    const parsed = parseMqtt(payload);
    if (parsed) {
      return { language: parsed.payloadLanguage, formatted: parsed.payload };
    }
    return { language: "plaintext", formatted: payload };
  }

  // Plain - 기존 로직
  if (isBinary) {
    return { language: "plaintext", formatted: payload };
  }

  try {
    const parsed = JSON.parse(payload);
    return { language: "json", formatted: JSON.stringify(parsed, null, 2) };
  } catch {
    // noop
  }

  if (payload.trimStart().startsWith("<")) {
    return { language: "xml", formatted: payload };
  }

  return { language: "plaintext", formatted: payload };
}
