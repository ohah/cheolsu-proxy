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

function decodeMqttUtf8String(bytes: Uint8Array, offset: number): [string, number] {
  if (offset + 2 > bytes.length) return ["", offset];
  const len = (bytes[offset] << 8) | bytes[offset + 1];
  const str = new TextDecoder().decode(bytes.slice(offset + 2, offset + 2 + len));
  return [str, offset + 2 + len];
}

export function formatMqtt(base64Payload: string): string {
  try {
    const bytes = decodeBase64ToBytes(base64Payload);
    if (bytes.length < 2) return base64Payload;

    const packetType = bytes[0] >> 4;
    const flags = bytes[0] & 0x0f;
    const typeName = MQTT_PACKET_TYPES[packetType] ?? `UNKNOWN(${packetType})`;
    const [remainingLength, payloadStart] = decodeMqttRemainingLength(bytes, 1);
    const lines: string[] = [`# MQTT ${typeName}`];

    if (packetType === 3) {
      // PUBLISH
      const dup = (flags & 0x08) !== 0;
      const qos = (flags & 0x06) >> 1;
      const retain = (flags & 0x01) !== 0;

      let offset = payloadStart;
      const [topic, newOffset] = decodeMqttUtf8String(bytes, offset);
      offset = newOffset;

      // Packet ID for QoS > 0
      let packetId: number | undefined;
      if (qos > 0 && offset + 2 <= bytes.length) {
        packetId = (bytes[offset] << 8) | bytes[offset + 1];
        offset += 2;
      }

      lines.push(`Topic: ${topic}`);
      lines.push(`QoS: ${qos}`);
      lines.push(`Retain: ${retain}`);
      if (dup) lines.push(`DUP: true`);
      if (packetId !== undefined) lines.push(`Packet ID: ${packetId}`);

      // Payload
      const payloadBytes = bytes.slice(offset, payloadStart + remainingLength);
      if (payloadBytes.length > 0) {
        const payloadText = new TextDecoder("utf-8", { fatal: false }).decode(payloadBytes);
        try {
          const json = JSON.parse(payloadText);
          lines.push(`Payload:\n${JSON.stringify(json, null, 2)}`);
        } catch {
          lines.push(`Payload:\n${payloadText}`);
        }
      }
    } else if (packetType === 1) {
      // CONNECT
      let offset = payloadStart;
      const [protocolName, off1] = decodeMqttUtf8String(bytes, offset);
      offset = off1;
      lines.push(`Protocol: ${protocolName}`);
      if (offset < bytes.length) {
        lines.push(`Version: ${bytes[offset]}`);
        offset += 1;
      }
      if (offset < bytes.length) {
        const connectFlags = bytes[offset];
        lines.push(`Clean Session: ${(connectFlags & 0x02) !== 0}`);
        offset += 1;
      }
      if (offset + 2 <= bytes.length) {
        const keepAlive = (bytes[offset] << 8) | bytes[offset + 1];
        lines.push(`Keep Alive: ${keepAlive}s`);
        offset += 2;
      }
      const [clientId] = decodeMqttUtf8String(bytes, offset);
      if (clientId) lines.push(`Client ID: ${clientId}`);
    } else if (packetType === 2) {
      // CONNACK
      if (payloadStart + 1 < bytes.length) {
        lines.push(`Session Present: ${(bytes[payloadStart] & 0x01) !== 0}`);
        lines.push(`Reason Code: ${bytes[payloadStart + 1]}`);
      }
    } else if (packetType === 8) {
      // SUBSCRIBE
      let offset = payloadStart;
      if (offset + 2 <= bytes.length) {
        const packetId = (bytes[offset] << 8) | bytes[offset + 1];
        lines.push(`Packet ID: ${packetId}`);
        offset += 2;
      }
      const subs: string[] = [];
      while (offset < payloadStart + remainingLength) {
        const [topic, newOff] = decodeMqttUtf8String(bytes, offset);
        offset = newOff;
        const qos = offset < bytes.length ? bytes[offset] : 0;
        offset += 1;
        subs.push(`  ${topic} (QoS ${qos})`);
      }
      if (subs.length > 0) lines.push(`Subscriptions:\n${subs.join("\n")}`);
    } else if (packetType === 9) {
      // SUBACK
      let offset = payloadStart;
      if (offset + 2 <= bytes.length) {
        const packetId = (bytes[offset] << 8) | bytes[offset + 1];
        lines.push(`Packet ID: ${packetId}`);
        offset += 2;
      }
      const codes: number[] = [];
      while (offset < payloadStart + remainingLength) {
        codes.push(bytes[offset]);
        offset += 1;
      }
      if (codes.length > 0) lines.push(`Return Codes: ${codes.join(", ")}`);
    }

    return lines.join("\n");
  } catch {
    return base64Payload;
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
    return { language: "mqtt", formatted: formatMqtt(payload) };
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
