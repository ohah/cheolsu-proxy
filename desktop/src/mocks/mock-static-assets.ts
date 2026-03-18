import type { HttpTransaction } from "@/entities/proxy";

import { textBody, tx } from "./mock-helpers";

const htmlSnippet = `<!DOCTYPE html><html><head><title>Example</title></head><body><h1>Hello World</h1></body></html>`;
const cssSnippet = `body { margin: 0; padding: 0; font-family: sans-serif; }`;
const jsSnippet = `(function(){console.log("loaded");})();`;

/** 정적 리소스 트랜잭션 (HTML, CSS, JS, Image) */
export const MOCK_STATIC_ASSET_TRANSACTIONS: HttpTransaction[] = [
  // 6. HTML page
  tx(
    {
      method: "GET",
      uri: "https://www.example.com/",
      headers: { accept: "text/html", "user-agent": "Mozilla/5.0" },
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 200,
      headers: {
        "content-type": "text/html; charset=utf-8",
        "cache-control": "max-age=3600",
      },
      ...textBody(htmlSnippet, "Html"),
    },
  ),

  // 7. CSS
  tx(
    {
      method: "GET",
      uri: "https://www.example.com/assets/style.css",
      headers: { accept: "text/css" },
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 200,
      headers: { "content-type": "text/css", "cache-control": "max-age=86400" },
      ...textBody(cssSnippet, "Css"),
    },
  ),

  // 8. JavaScript
  tx(
    {
      method: "GET",
      uri: "https://www.example.com/assets/app.bundle.js",
      headers: { accept: "*/*" },
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 200,
      headers: { "content-type": "application/javascript" },
      ...textBody(jsSnippet, "Javascript"),
    },
  ),

  // 9. Image (PNG)
  tx(
    {
      method: "GET",
      uri: "https://cdn.example.com/images/logo.png",
      headers: { accept: "image/*" },
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 200,
      headers: { "content-type": "image/png", "content-length": "24680" },
      data_type: "Image",
      body_size: 24680,
    },
  ),
];
