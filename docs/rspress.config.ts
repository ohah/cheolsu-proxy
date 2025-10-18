import { defineConfig } from "rspress/config";

export default defineConfig({
  root: ".",
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
      },
      {
        lang: "en",
        label: "English",
        outlineTitle: "ON THIS PAGE",
        prevPageText: "Previous Page",
        nextPageText: "Next Page",
      },
    ],
    socialLinks: [
      {
        icon: "github",
        mode: "link",
        content: "https://github.com/ohah/cheolsu-proxy",
      },
    ],
    nav: [
      {
        text: "사용하기",
        link: "/usage/",
      },
      {
        text: "가이드",
        link: "/guide/getting-started",
      },
      {
        text: "기여하기",
        link: "/contributing/",
      },
    ],
    sidebar: {
      "/usage/": [
        {
          text: "사용하기",
          items: [
            {
              text: "기본 사용법",
              link: "/usage/basic-usage",
            },
            {
              text: "프록시 설정",
              link: "/usage/proxy-setup",
            },
            {
              text: "인증서 설치",
              link: "/usage/certificate-setup",
            },
            {
              text: "문제 해결",
              link: "/usage/troubleshooting",
            },
          ],
        },
      ],
      "/en/usage/": [
        {
          text: "Usage",
          items: [
            {
              text: "Basic Usage",
              link: "/en/usage/basic-usage",
            },
            {
              text: "Proxy Setup",
              link: "/en/usage/proxy-setup",
            },
            {
              text: "Certificate Setup",
              link: "/en/usage/certificate-setup",
            },
            {
              text: "Troubleshooting",
              link: "/en/usage/troubleshooting",
            },
          ],
        },
      ],
      "/guide/": [
        {
          text: "gettingStarted",
          items: [
            {
              text: "introduction",
              link: "/guide/getting-started",
            },
            {
              text: "features",
              link: "/guide/features",
            },
          ],
        },
      ],
      "/en/guide/": [
        {
          text: "gettingStarted",
          items: [
            {
              text: "introduction",
              link: "/en/guide/getting-started",
            },
            {
              text: "features",
              link: "/en/guide/features",
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
