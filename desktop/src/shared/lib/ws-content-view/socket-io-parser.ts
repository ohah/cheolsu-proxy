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
