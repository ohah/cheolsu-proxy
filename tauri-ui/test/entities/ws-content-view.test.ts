import { describe, test, expect } from "bun:test";
import {
  formatSocketIO,
  parseMqtt,
  getMqttSummary,
  getWsContentView,
} from "../../src/shared/lib/ws-content-view";

// --- Helper: bytes를 base64로 인코딩 ---
function toBase64(bytes: number[]): string {
  return btoa(String.fromCharCode(...bytes));
}

// --- Helper: MQTT UTF-8 string을 바이트 배열로 인코딩 ---
function mqttString(str: string): number[] {
  const encoded = new TextEncoder().encode(str);
  return [(encoded.length >> 8) & 0xff, encoded.length & 0xff, ...encoded];
}

// --- Helper: MQTT Remaining Length를 Variable Byte Integer로 인코딩 ---
function encodeRemainingLength(value: number): number[] {
  const bytes: number[] = [];
  let v = value;
  do {
    let byte = v % 128;
    v = Math.floor(v / 128);
    if (v > 0) byte |= 0x80;
    bytes.push(byte);
  } while (v > 0);
  return bytes;
}

// --- Helper: MQTT PUBLISH 패킷 생성 ---
function buildPublishPacket(
  topic: string,
  payload: string,
  options?: { qos?: number; retain?: boolean; dup?: boolean; mqttV5Props?: number[] },
): number[] {
  const qos = options?.qos ?? 0;
  const retain = options?.retain ?? false;
  const dup = options?.dup ?? false;

  const topicBytes = mqttString(topic);
  const payloadBytes = [...new TextEncoder().encode(payload)];
  const packetIdBytes = qos > 0 ? [0x00, 0x01] : [];
  const propsBytes = options?.mqttV5Props ?? [];

  const varHeader = [...topicBytes, ...packetIdBytes, ...propsBytes, ...payloadBytes];
  const remainingLength = encodeRemainingLength(varHeader.length);

  let flags = 0x30; // PUBLISH = 3 << 4
  flags |= (qos & 0x03) << 1;
  if (retain) flags |= 0x01;
  if (dup) flags |= 0x08;

  return [flags, ...remainingLength, ...varHeader];
}

// --- Helper: MQTT CONNECT 패킷 생성 ---
function buildConnectPacket(version: number): number[] {
  const protocolName = version === 5 ? mqttString("MQTT") : mqttString("MQTT");
  const versionByte = version;
  const connectFlags = 0x02; // Clean Session
  const keepAlive = [0x00, 0x3c]; // 60s
  const clientId = mqttString("test-client");

  const varHeader = [...protocolName, versionByte, connectFlags, ...keepAlive, ...clientId];
  const remainingLength = encodeRemainingLength(varHeader.length);

  return [0x10, ...remainingLength, ...varHeader]; // CONNECT = 1 << 4
}

// --- Helper: MQTT SUBSCRIBE 패킷 생성 ---
function buildSubscribePacket(
  topics: { topic: string; qos: number }[],
  options?: { mqttV5Props?: number[] },
): number[] {
  const packetId = [0x00, 0x0a]; // Packet ID = 10
  const propsBytes = options?.mqttV5Props ?? [];
  const topicBytes = topics.flatMap((t) => [...mqttString(t.topic), t.qos]);

  const varHeader = [...packetId, ...propsBytes, ...topicBytes];
  const remainingLength = encodeRemainingLength(varHeader.length);

  return [0x82, ...remainingLength, ...varHeader]; // SUBSCRIBE = 8 << 4 | 0x02
}

// --- Helper: MQTT CONNACK 패킷 생성 ---
function buildConnackPacket(sessionPresent: boolean, reasonCode: number): number[] {
  const sp = sessionPresent ? 0x01 : 0x00;
  return [0x20, 0x02, sp, reasonCode]; // CONNACK = 2 << 4
}

// ============================================================
// Socket.IO 파싱 테스트
// ============================================================

describe("formatSocketIO", () => {
  test("EVENT 메시지를 파싱한다", () => {
    const result = formatSocketIO('42["chat","hello world"]');
    expect(result).toContain("Socket.IO EVENT");
    expect(result).toContain("Event: chat");
    expect(result).toContain("hello world");
  });

  test("CONNECT 메시지를 파싱한다", () => {
    const result = formatSocketIO("40");
    expect(result).toContain("Socket.IO CONNECT");
  });

  test("DISCONNECT 메시지를 파싱한다", () => {
    const result = formatSocketIO("41");
    expect(result).toContain("Socket.IO DISCONNECT");
  });

  test("ACK 메시지를 파싱한다", () => {
    const result = formatSocketIO('43[{"status":"ok"}]');
    expect(result).toContain("Socket.IO ACK");
  });

  test("네임스페이스가 있는 EVENT를 파싱한다", () => {
    const result = formatSocketIO('42/admin,["update",{"id":1}]');
    expect(result).toContain("Namespace: /admin");
    expect(result).toContain("Event: update");
  });

  test("Ack ID가 있는 EVENT를 파싱한다", () => {
    const result = formatSocketIO('4215["chat","hi"]');
    expect(result).toContain("Ack ID: 15");
    expect(result).toContain("Event: chat");
  });

  test("CONNECT_ERROR를 파싱한다", () => {
    const result = formatSocketIO('44{"message":"unauthorized"}');
    expect(result).toContain("Socket.IO CONNECT_ERROR");
  });

  test("짧은 페이로드는 그대로 반환한다", () => {
    const result = formatSocketIO("4");
    expect(result).toBe("4");
  });

  test("여러 데이터 인자를 가진 EVENT를 파싱한다", () => {
    const result = formatSocketIO('42["msg","hello","world"]');
    expect(result).toContain("Event: msg");
    // 여러 인자는 배열로 표시
    expect(result).toContain("hello");
    expect(result).toContain("world");
  });
});

// ============================================================
// MQTT parseMqtt 테스트
// ============================================================

describe("parseMqtt", () => {
  test("PUBLISH 패킷을 파싱한다", () => {
    const packet = buildPublishPacket("sensor/temp", '{"value":23.5}');
    const result = parseMqtt(toBase64(packet));

    expect(result).not.toBeNull();
    expect(result!.meta.packetType).toBe("PUBLISH");
    expect(result!.meta.fields.find((f) => f.label === "Topic")?.value).toBe("sensor/temp");
    expect(result!.meta.fields.find((f) => f.label === "QoS")?.value).toBe("0");
    expect(result!.meta.fields.find((f) => f.label === "Retain")?.value).toBe("false");
    // JSON payload는 pretty-print
    expect(result!.payloadLanguage).toBe("json");
    expect(result!.payload).toContain('"value": 23.5');
  });

  test("QoS 1 PUBLISH 패킷의 Packet ID를 파싱한다", () => {
    const packet = buildPublishPacket("device/status", "online", { qos: 1 });
    const result = parseMqtt(toBase64(packet));

    expect(result).not.toBeNull();
    expect(result!.meta.fields.find((f) => f.label === "QoS")?.value).toBe("1");
    expect(result!.meta.fields.find((f) => f.label === "Packet ID")?.value).toBe("1");
  });

  test("Retain 플래그가 있는 PUBLISH를 파싱한다", () => {
    const packet = buildPublishPacket("config/mode", "auto", { retain: true });
    const result = parseMqtt(toBase64(packet));

    expect(result!.meta.fields.find((f) => f.label === "Retain")?.value).toBe("true");
  });

  test("DUP 플래그가 있는 PUBLISH를 파싱한다", () => {
    const packet = buildPublishPacket("test/dup", "data", { qos: 1, dup: true });
    const result = parseMqtt(toBase64(packet));

    expect(result!.meta.fields.find((f) => f.label === "DUP")?.value).toBe("true");
  });

  test("CONNECT 패킷을 파싱한다 (v3.1.1)", () => {
    const packet = buildConnectPacket(4);
    const result = parseMqtt(toBase64(packet));

    expect(result).not.toBeNull();
    expect(result!.meta.packetType).toBe("CONNECT");
    expect(result!.meta.fields.find((f) => f.label === "Protocol")?.value).toBe("MQTT");
    expect(result!.meta.fields.find((f) => f.label === "Version")?.value).toBe("4");
    expect(result!.meta.fields.find((f) => f.label === "Client ID")?.value).toBe("test-client");
    expect(result!.meta.fields.find((f) => f.label === "Clean Session")?.value).toBe("true");
  });

  test("CONNECT 패킷을 파싱한다 (v5.0)", () => {
    const packet = buildConnectPacket(5);
    const result = parseMqtt(toBase64(packet));

    expect(result!.meta.packetType).toBe("CONNECT");
    expect(result!.meta.fields.find((f) => f.label === "Version")?.value).toBe("5");
  });

  test("CONNACK 패킷을 파싱한다", () => {
    const packet = buildConnackPacket(false, 0);
    const result = parseMqtt(toBase64(packet));

    expect(result).not.toBeNull();
    expect(result!.meta.packetType).toBe("CONNACK");
    expect(result!.meta.fields.find((f) => f.label === "Session Present")?.value).toBe("false");
    expect(result!.meta.fields.find((f) => f.label === "Reason Code")?.value).toBe("0");
  });

  test("SUBSCRIBE 패킷을 파싱한다 (단일 토픽)", () => {
    const packet = buildSubscribePacket([{ topic: "home/living/temp", qos: 1 }]);
    const result = parseMqtt(toBase64(packet));

    expect(result).not.toBeNull();
    expect(result!.meta.packetType).toBe("SUBSCRIBE");
    expect(result!.meta.fields.find((f) => f.label === "Packet ID")?.value).toBe("10");
    expect(result!.meta.fields.find((f) => f.label === "Topic")?.value).toBe("home/living/temp");
    expect(result!.meta.fields.find((f) => f.label === "QoS")?.value).toBe("1");
  });

  test("SUBSCRIBE 패킷을 파싱한다 (다중 토픽)", () => {
    const packet = buildSubscribePacket([
      { topic: "sensor/temp", qos: 0 },
      { topic: "sensor/humidity", qos: 1 },
      { topic: "device/status", qos: 2 },
    ]);
    const result = parseMqtt(toBase64(packet));

    expect(result).not.toBeNull();
    // 다중 토픽이면 Topic 1, Topic 2, Topic 3
    const topicFields = result!.meta.fields.filter((f) => f.label.startsWith("Topic"));
    expect(topicFields).toHaveLength(3);
    expect(topicFields[0].value).toBe("sensor/temp");
    expect(topicFields[1].value).toBe("sensor/humidity");
    expect(topicFields[2].value).toBe("device/status");
  });

  test("MQTT 5 SUBSCRIBE 패킷에서 Properties를 건너뛴다", () => {
    // Properties Length = 0 (최소)
    const packet = buildSubscribePacket([{ topic: "v5/topic", qos: 0 }], {
      mqttV5Props: [0x00], // Properties Length = 0
    });
    const result = parseMqtt(toBase64(packet), 5);

    expect(result).not.toBeNull();
    expect(result!.meta.packetType).toBe("SUBSCRIBE");
    expect(result!.meta.fields.find((f) => f.label === "Topic")?.value).toBe("v5/topic");
  });

  test("MQTT 5 PUBLISH 패킷에서 Properties를 건너뛴다", () => {
    const packet = buildPublishPacket("v5/data", '{"ok":true}', {
      mqttV5Props: [0x00], // Properties Length = 0
    });
    const result = parseMqtt(toBase64(packet), 5);

    expect(result).not.toBeNull();
    expect(result!.meta.packetType).toBe("PUBLISH");
    expect(result!.meta.fields.find((f) => f.label === "Topic")?.value).toBe("v5/data");
    expect(result!.payloadLanguage).toBe("json");
    expect(result!.payload).toContain('"ok": true');
  });

  test("PINGREQ 패킷을 파싱한다", () => {
    const packet = [0xc0, 0x00]; // PINGREQ = 12 << 4
    const result = parseMqtt(toBase64(packet));

    expect(result).not.toBeNull();
    expect(result!.meta.packetType).toBe("PINGREQ");
  });

  test("PINGRESP 패킷을 파싱한다", () => {
    const packet = [0xd0, 0x00]; // PINGRESP = 13 << 4
    const result = parseMqtt(toBase64(packet));

    expect(result).not.toBeNull();
    expect(result!.meta.packetType).toBe("PINGRESP");
  });

  test("DISCONNECT 패킷을 파싱한다", () => {
    const packet = [0xe0, 0x00]; // DISCONNECT = 14 << 4
    const result = parseMqtt(toBase64(packet));

    expect(result).not.toBeNull();
    expect(result!.meta.packetType).toBe("DISCONNECT");
  });

  test("너무 짧은 데이터는 null을 반환한다", () => {
    const result = parseMqtt(toBase64([0x30]));
    expect(result).toBeNull();
  });

  test("잘못된 base64는 null을 반환한다", () => {
    const result = parseMqtt("!!!invalid-base64!!!");
    expect(result).toBeNull();
  });

  test("PUBLISH payload가 plain text인 경우 언어가 plaintext이다", () => {
    const packet = buildPublishPacket("test/plain", "hello world");
    const result = parseMqtt(toBase64(packet));

    expect(result!.payloadLanguage).toBe("plaintext");
    expect(result!.payload).toBe("hello world");
  });
});

// ============================================================
// MQTT getMqttSummary 테스트 (테이블 표시용)
// ============================================================

describe("getMqttSummary", () => {
  test("PUBLISH 패킷에서 packetType과 topic을 반환한다", () => {
    const packet = buildPublishPacket("home/temp", "25.3");
    const result = getMqttSummary(toBase64(packet));

    expect(result).not.toBeNull();
    expect(result!.packetType).toBe("PUBLISH");
    expect(result!.topic).toBe("home/temp");
  });

  test("SUBSCRIBE 패킷에서 첫 번째 topic을 반환한다", () => {
    const packet = buildSubscribePacket([
      { topic: "device/#", qos: 1 },
      { topic: "sensor/#", qos: 0 },
    ]);
    const result = getMqttSummary(toBase64(packet));

    expect(result).not.toBeNull();
    expect(result!.packetType).toBe("SUBSCRIBE");
    expect(result!.topic).toBe("device/#");
  });

  test("MQTT 5 SUBSCRIBE에서 Properties를 건너뛰고 topic을 반환한다", () => {
    const packet = buildSubscribePacket([{ topic: "v5/sensors/+", qos: 0 }], {
      mqttV5Props: [0x00], // Properties Length = 0
    });
    const result = getMqttSummary(toBase64(packet), 5);

    expect(result).not.toBeNull();
    expect(result!.topic).toBe("v5/sensors/+");
  });

  test("CONNACK 패킷은 topic이 null이다", () => {
    const packet = buildConnackPacket(false, 0);
    const result = getMqttSummary(toBase64(packet));

    expect(result).not.toBeNull();
    expect(result!.packetType).toBe("CONNACK");
    expect(result!.topic).toBeNull();
  });

  test("PINGREQ는 topic이 null이다", () => {
    const result = getMqttSummary(toBase64([0xc0, 0x00]));

    expect(result).not.toBeNull();
    expect(result!.packetType).toBe("PINGREQ");
    expect(result!.topic).toBeNull();
  });

  test("잘못된 패킷은 null을 반환한다", () => {
    expect(getMqttSummary(toBase64([0xf0, 0x00]))).toBeNull(); // type 15는 유효하지 않음
  });

  test("빈 데이터는 null을 반환한다", () => {
    expect(getMqttSummary(toBase64([]))).toBeNull();
  });
});

// ============================================================
// getWsContentView 통합 테스트
// ============================================================

describe("getWsContentView", () => {
  test("Socket.IO 메시지를 socketio 언어로 반환한다", () => {
    const { language, formatted } = getWsContentView('42["event","data"]', false, "socket_io");
    expect(language).toBe("socketio");
    expect(formatted).toContain("Socket.IO EVENT");
  });

  test("MQTT 메시지에서 payload만 반환한다", () => {
    const packet = buildPublishPacket("test/topic", '{"temp":22}');
    const { language, formatted } = getWsContentView(toBase64(packet), true, "mqtt");
    expect(language).toBe("json");
    expect(formatted).toContain('"temp": 22');
    // 메타 정보(Topic 등)는 포함되지 않아야 함
    expect(formatted).not.toContain("test/topic");
  });

  test("MQTT payload가 없는 패킷은 빈 문자열을 반환한다", () => {
    const packet = [0xc0, 0x00]; // PINGREQ
    const { language, formatted } = getWsContentView(toBase64(packet), true, "mqtt");
    expect(language).toBe("plaintext");
    expect(formatted).toBe("");
  });

  test("plain JSON 텍스트는 json 언어로 포맷한다", () => {
    const { language, formatted } = getWsContentView('{"key":"value"}', false);
    expect(language).toBe("json");
    expect(formatted).toContain('"key": "value"');
  });

  test("plain XML 텍스트는 xml 언어로 반환한다", () => {
    const { language } = getWsContentView("<root><item/></root>", false);
    expect(language).toBe("xml");
  });

  test("일반 텍스트는 plaintext로 반환한다", () => {
    const { language, formatted } = getWsContentView("hello world", false);
    expect(language).toBe("plaintext");
    expect(formatted).toBe("hello world");
  });

  test("바이너리(plain)는 plaintext로 반환한다", () => {
    const { language } = getWsContentView("SGVsbG8=", true);
    expect(language).toBe("plaintext");
  });
});
