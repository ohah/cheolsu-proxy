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
    locales: [
      {
        lang: "ko",
        outlineTitle: "이 페이지에서",
        prevPageText: "이전 페이지",
        nextPageText: "다음 페이지",
      },
      {
        lang: "en",
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
        text: "guide",
        link: "/guide/getting-started",
      },
      {
        text: "contributing",
        link: "/contributing/",
      },
    ],
    sidebar: {
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
            {
              text: "certificateSetup",
              link: "/guide/certificate-setup",
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
            {
              text: "certificateSetup",
              link: "/en/guide/certificate-setup",
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
  },
});
