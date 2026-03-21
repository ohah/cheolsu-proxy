import { defineConfig } from "@rspress/core";

export default defineConfig({
  root: ".",
  ssg: {
    experimentalExcludeRoutePaths: ["/rspress.config"],
  },
  base: process.env.NODE_ENV === "production" ? "/cheolsu-proxy/" : "/",
  title: "Cheolsu Proxy",
  description: "A simple Man In The Middle proxy built with Rust and Tauri",
  lang: "ko",
  logo: "/logo.png",
  logoText: "Cheolsu Proxy",
  locales: [
    {
      lang: "ko",
      label: "한국어",
      title: "Cheolsu Proxy",
      description: "Rust와 Tauri로 구축된 간단한 Man In The Middle 프록시",
    },
    {
      lang: "en",
      label: "English",
      title: "Cheolsu Proxy",
      description: "A simple Man In The Middle proxy built with Rust and Tauri",
    },
  ],
  themeConfig: {
    locales: [
      {
        lang: "ko",
        label: "한국어",
        outlineTitle: "이 페이지에서",
        prevPageText: "이전 페이지",
        nextPageText: "다음 페이지",
        nav: [
          {
            text: "가이드",
            link: "/guide/",
          },
          {
            text: "기능",
            link: "/features/mcp-server",
          },
          {
            text: "기여하기",
            link: "/contributing/",
          },
          {
            text: "릴리즈 노트",
            link: "/releases/",
          },
        ],
      },
      {
        lang: "en",
        label: "English",
        outlineTitle: "ON THIS PAGE",
        prevPageText: "Previous Page",
        nextPageText: "Next Page",
        nav: [
          {
            text: "Guide",
            link: "/en/guide/",
          },
          {
            text: "Features",
            link: "/en/features/mcp-server",
          },
          {
            text: "Contributing",
            link: "/en/contributing/",
          },
          {
            text: "Release Notes",
            link: "/en/releases/",
          },
        ],
      },
    ],
    socialLinks: [
      {
        icon: "github",
        mode: "link",
        content: "https://github.com/ohah/cheolsu-proxy",
      },
    ],
    sidebar: {
      "/features/": [
        {
          text: "기능",
          items: [
            {
              text: "MCP Server",
              link: "/features/mcp-server",
            },
            {
              text: "Cheolsu Query",
              link: "/features/cheolsu-query",
            },
            {
              text: "인터셉트 규칙",
              link: "/features/intercept-rules",
            },
            {
              text: "스크립팅",
              link: "/features/scripting",
            },
            {
              text: "WebSocket",
              link: "/features/websocket",
            },
            {
              text: "Server Replay",
              link: "/features/server-replay",
            },
            {
              text: "TLS 지원",
              link: "/features/tls-support",
            },
            {
              text: "Breakpoint",
              link: "/features/breakpoint",
            },
            {
              text: "네트워크 스로틀링",
              link: "/features/network-throttling",
            },
            {
              text: "Server-Sent Events",
              link: "/features/sse",
            },
            {
              text: "GraphQL",
              link: "/features/graphql",
            },
            {
              text: "Host Mapping",
              link: "/features/host-mapping",
            },
            {
              text: "Reverse Proxy",
              link: "/features/reverse-proxy",
            },
            {
              text: "Traffic Diff",
              link: "/features/traffic-diff",
            },
            {
              text: "트래픽 분석",
              link: "/features/analytics",
            },
            {
              text: "Contract Testing",
              link: "/features/contract-testing",
            },
            {
              text: "gRPC / Protobuf",
              link: "/features/grpc",
            },
          ],
        },
      ],
      "/en/features/": [
        {
          text: "Features",
          items: [
            {
              text: "MCP Server",
              link: "/en/features/mcp-server",
            },
            {
              text: "Cheolsu Query",
              link: "/en/features/cheolsu-query",
            },
            {
              text: "Intercept Rules",
              link: "/en/features/intercept-rules",
            },
            {
              text: "Scripting",
              link: "/en/features/scripting",
            },
            {
              text: "WebSocket",
              link: "/en/features/websocket",
            },
            {
              text: "Server Replay",
              link: "/en/features/server-replay",
            },
            {
              text: "TLS Support",
              link: "/en/features/tls-support",
            },
            {
              text: "Breakpoint",
              link: "/en/features/breakpoint",
            },
            {
              text: "Network Throttling",
              link: "/en/features/network-throttling",
            },
            {
              text: "Server-Sent Events",
              link: "/en/features/sse",
            },
            {
              text: "GraphQL",
              link: "/en/features/graphql",
            },
            {
              text: "Host Mapping",
              link: "/en/features/host-mapping",
            },
            {
              text: "Reverse Proxy",
              link: "/en/features/reverse-proxy",
            },
            {
              text: "Traffic Diff",
              link: "/en/features/traffic-diff",
            },
            {
              text: "Traffic Analytics",
              link: "/en/features/analytics",
            },
            {
              text: "Contract Testing",
              link: "/en/features/contract-testing",
            },
            {
              text: "gRPC / Protobuf",
              link: "/en/features/grpc",
            },
          ],
        },
      ],
      "/guide/": [
        {
          text: "가이드",
          items: [
            {
              text: "설치",
              link: "/guide/installation",
            },
            {
              text: "기본 사용법",
              link: "/guide/basic-usage",
            },
            {
              text: "SSL 인증서",
              link: "/guide/ssl-certificates",
            },
            {
              text: "프록시 설정",
              link: "/guide/proxying",
            },
            {
              text: "트래픽 기록",
              link: "/guide/recording",
            },
            {
              text: "세션 관리",
              link: "/guide/sessions",
            },
            {
              text: "CLI",
              link: "/guide/cli",
            },
            {
              text: "문제 해결",
              link: "/guide/troubleshooting",
            },
          ],
        },
      ],
      "/en/guide/": [
        {
          text: "Guide",
          items: [
            {
              text: "Installation",
              link: "/en/guide/installation",
            },
            {
              text: "Basic Usage",
              link: "/en/guide/basic-usage",
            },
            {
              text: "SSL Certificates",
              link: "/en/guide/ssl-certificates",
            },
            {
              text: "Proxying",
              link: "/en/guide/proxying",
            },
            {
              text: "Recording",
              link: "/en/guide/recording",
            },
            {
              text: "Sessions",
              link: "/en/guide/sessions",
            },
            {
              text: "CLI",
              link: "/en/guide/cli",
            },
            {
              text: "Troubleshooting",
              link: "/en/guide/troubleshooting",
            },
          ],
        },
      ],
      "/releases/": [
        {
          text: "릴리즈 노트",
          items: [
            {
              text: "릴리즈 노트",
              link: "/releases/",
            },
          ],
        },
      ],
      "/en/releases/": [
        {
          text: "Release Notes",
          items: [
            {
              text: "Release Notes",
              link: "/en/releases/",
            },
          ],
        },
      ],
      "/contributing/": [
        {
          text: "기여하기",
          items: [
            {
              text: "기여하기",
              link: "/contributing/",
            },
            {
              text: "개발 환경 설정",
              link: "/contributing/development-setup",
            },
            {
              text: "프로젝트 구조",
              link: "/contributing/code-structure",
            },
            {
              text: "테스트",
              link: "/contributing/testing",
            },
          ],
        },
      ],
      "/en/contributing/": [
        {
          text: "Contributing",
          items: [
            {
              text: "Contributing",
              link: "/en/contributing/",
            },
            {
              text: "Development Setup",
              link: "/en/contributing/development-setup",
            },
            {
              text: "Code Structure",
              link: "/en/contributing/code-structure",
            },
            {
              text: "Testing",
              link: "/en/contributing/testing",
            },
          ],
        },
      ],
    },
  },
  builderConfig: {
    output: {
      distPath: {
        root: "doc_build",
      },
    },
    dev: {
      client: {
        overlay: false,
      },
    },
  },
});
