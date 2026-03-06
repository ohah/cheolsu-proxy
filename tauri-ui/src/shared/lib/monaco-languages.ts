import type { Monaco } from "@monaco-editor/react";

let isRegistered = false;

export const registerContentViewLanguages = (monaco: Monaco): void => {
  if (isRegistered) return;

  // --- GraphQL ---
  monaco.languages.register({ id: "graphql" });
  monaco.languages.setMonarchTokensProvider("graphql", {
    keywords: [
      "query",
      "mutation",
      "subscription",
      "fragment",
      "on",
      "type",
      "input",
      "enum",
      "scalar",
      "interface",
      "union",
      "extend",
      "implements",
      "directive",
      "schema",
      "true",
      "false",
      "null",
    ],
    typeKeywords: ["Int", "Float", "String", "Boolean", "ID"],
    operators: ["=", "!", "|", "&", "..."],
    symbols: /[=!|&.]+/,
    tokenizer: {
      root: [
        [/#.*$/, "comment"],
        [/"([^"\\]|\\.)*"/, "string"],
        [/"""/, "string", "@multilineString"],
        [/\$\w+/, "variable"],
        [/@\w+/, "annotation"],
        [
          /[a-zA-Z_]\w*/,
          {
            cases: {
              "@keywords": "keyword",
              "@typeKeywords": "type",
              "@default": "identifier",
            },
          },
        ],
        [/[{}()[\]]/, "delimiter.bracket"],
        [/:/, "delimiter"],
        [/[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?/, "number"],
        [/[,]/, "delimiter"],
      ],
      multilineString: [
        [/"""/, "string", "@pop"],
        [/./, "string"],
      ],
    },
  });
  monaco.languages.setLanguageConfiguration("graphql", {
    comments: { lineComment: "#" },
    brackets: [
      ["{", "}"],
      ["(", ")"],
      ["[", "]"],
    ],
    autoClosingPairs: [
      { open: "{", close: "}" },
      { open: "(", close: ")" },
      { open: "[", close: "]" },
      { open: '"', close: '"' },
    ],
  });

  // --- Socket.IO ---
  monaco.languages.register({ id: "socketio" });
  monaco.languages.setMonarchTokensProvider("socketio", {
    tokenizer: {
      root: [
        [/^#\s.*$/, "comment"],
        [/^(Event|Namespace|Ack ID|Type):/, "keyword"],
        [/"([^"\\]|\\.)*"/, "string"],
        [/\b(CONNECT|DISCONNECT|EVENT|ACK|CONNECT_ERROR)\b/, "type"],
        [/[{}[\],]/, "delimiter"],
        [/:/, "delimiter"],
        [/\b\d+(\.\d+)?\b/, "number"],
        [/\b(true|false|null)\b/, "keyword"],
      ],
    },
  });

  // --- MQTT ---
  monaco.languages.register({ id: "mqtt" });
  monaco.languages.setMonarchTokensProvider("mqtt", {
    tokenizer: {
      root: [
        [/^#\s.*$/, "comment"],
        [
          /^(Topic|QoS|Retain|Packet ID|Client ID|Protocol|Version|Keep Alive|Clean Session|Will Topic|Will QoS|Will Retain|Username|Reason Code|Session Present|Return Codes|Subscriptions):/,
          "keyword",
        ],
        [
          /\b(CONNECT|CONNACK|PUBLISH|PUBACK|PUBREC|PUBREL|PUBCOMP|SUBSCRIBE|SUBACK|UNSUBSCRIBE|UNSUBACK|PINGREQ|PINGRESP|DISCONNECT)\b/,
          "type",
        ],
        [/"([^"\\]|\\.)*"/, "string"],
        [/[{}[\],]/, "delimiter"],
        [/:/, "delimiter"],
        [/\b\d+(\.\d+)?\b/, "number"],
        [/\b(true|false|null)\b/, "keyword"],
      ],
    },
  });

  isRegistered = true;
};
