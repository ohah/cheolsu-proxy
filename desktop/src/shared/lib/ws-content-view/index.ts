import type { WsContentType } from "@/entities/websocket/model/types";

import { formatSocketIO } from "./socket-io-parser";
import { parseMqtt } from "./mqtt-parser";

export { formatSocketIO } from "./socket-io-parser";
export type { MqttMeta, MqttParsed } from "./mqtt-parser";
export { parseMqtt, getMqttSummary } from "./mqtt-parser";

/**
 * WebSocket content type에 따라 Monaco 언어와 포맷된 텍스트를 반환
 */
export function getWsContentView(
  payload: string,
  isBinary: boolean,
  contentType?: WsContentType,
  mqttVersion?: number,
): { language: string; formatted: string } {
  if (contentType === "socket_io") {
    return { language: "socketio", formatted: formatSocketIO(payload) };
  }

  if (contentType === "mqtt") {
    const parsed = parseMqtt(payload, mqttVersion);
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
