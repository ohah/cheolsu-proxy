import { defineConfig } from "rspress/config";

export default defineConfig({
  root: ".",
  base: process.env.NODE_ENV === "production" ? "/cheolsu-proxy/" : "/",
  title: "Cheolsu Proxy",
  description: "A simple Man In The Middle proxy built with Rust and Tauri",
  lang: "ko",
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
    socialLinks: [
      {
        icon: "github",
        mode: "link",
        content: "https://github.com/ohah/cheolsu-proxy",
      },
    ],
    nav: [
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
      "/guide/": [
        {
          text: "시작하기",
          items: [
            {
              text: "소개",
              link: "/guide/getting-started",
            },
            {
              text: "주요 기능",
              link: "/guide/features",
            },
            {
              text: "인증서 설정",
              link: "/guide/certificate-setup",
            },
          ],
        },
      ],
      "/contributing/": [
        {
          text: "기여하기",
          items: [
            {
              text: "기여 방법",
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
      "/features/": [
        {
          text: "기능",
          items: [
            {
              text: "TLS 1.0/1.1 지원",
              link: "/features/tls-support",
            },
          ],
        },
      ],
    },
  },
});
