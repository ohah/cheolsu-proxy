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
      // _meta.json 파일을 사용하여 사이드바 구조 자동 생성
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
