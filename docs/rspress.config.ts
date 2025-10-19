import { defineConfig } from "rspress/config";
import pluginMermaid from "rspress-plugin-mermaid";
import { pluginNodePolyfill } from "@rsbuild/plugin-node-polyfill";

export default defineConfig({
  root: ".",
  base: process.env.NODE_ENV === "production" ? "/cheolsu-proxy/" : "/",
  title: "Cheolsu Proxy",
  description: "A simple Man In The Middle proxy built with Rust and Tauri",
  lang: "ko",
  logo: "/logo.png",
  logoText: "Cheolsu Proxy",
  plugins: [pluginMermaid(), pluginNodePolyfill()],
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
      "/guide/": [
        {
          text: "가이드",
          items: [
            {
              text: "기본 사용법",
              link: "/guide/basic-usage",
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
              text: "Basic Usage",
              link: "/en/guide/basic-usage",
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
            {
              text: "지원 예정",
              link: "/releases/roadmap",
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
            {
              text: "Planned Features",
              link: "/en/releases/roadmap",
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
