import { defineConfig } from "rspress/config";
import pluginMermaid from "rspress-plugin-mermaid";

export default defineConfig({
  root: ".",
  base: process.env.NODE_ENV === "production" ? "/proxelar/" : "/",
  title: "Cheolsu Proxy",
  description: "A simple Man In The Middle proxy built with Rust and Tauri",
  lang: "ko",
  logo: "/logo.png",
  logoText: "Cheolsu Proxy",
  route: {
    cleanUrls: true,
  },
  plugins: [pluginMermaid()],
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
          text: "contributing",
          items: [
            {
              text: "contributingGuide",
              link: "/contributing/",
            },
            {
              text: "developmentSetup",
              link: "/contributing/development-setup",
            },
            {
              text: "codeStructure",
              link: "/contributing/code-structure",
            },
            {
              text: "testing",
              link: "/contributing/testing",
            },
          ],
        },
      ],
      "/en/contributing/": [
        {
          text: "contributing",
          items: [
            {
              text: "contributingGuide",
              link: "/en/contributing/",
            },
            {
              text: "developmentSetup",
              link: "/en/contributing/development-setup",
            },
            {
              text: "codeStructure",
              link: "/en/contributing/code-structure",
            },
            {
              text: "testing",
              link: "/en/contributing/testing",
            },
          ],
        },
      ],
      "/features/": [
        {
          text: "featureSection",
          items: [
            {
              text: "tlsSupport",
              link: "/features/tls-support",
            },
          ],
        },
      ],
      "/en/features/": [
        {
          text: "featureSection",
          items: [
            {
              text: "tlsSupport",
              link: "/en/features/tls-support",
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
