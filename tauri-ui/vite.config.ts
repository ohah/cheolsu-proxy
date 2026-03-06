import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

const host = process.env.TAURI_DEV_HOST;
const isMock = process.env.MOCK === "true";

const mockAliases = isMock
  ? {
      "@tauri-apps/api/core": path.resolve(__dirname, "./src/mocks/tauri-api-core.ts"),
      "@tauri-apps/api/event": path.resolve(__dirname, "./src/mocks/tauri-api-event.ts"),
      "@tauri-apps/plugin-fs": path.resolve(__dirname, "./src/mocks/tauri-plugin-fs.ts"),
      "@tauri-apps/plugin-store": path.resolve(__dirname, "./src/mocks/tauri-plugin-store.ts"),
      "@tauri-apps/plugin-clipboard-manager": path.resolve(__dirname, "./src/mocks/tauri-plugin-clipboard-manager.ts"),
      "@tauri-apps/plugin-http": path.resolve(__dirname, "./src/mocks/tauri-plugin-http.ts"),
      "@tauri-apps/plugin-opener": path.resolve(__dirname, "./src/mocks/tauri-plugin-opener.ts"),
    }
  : {};

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      ...mockAliases,
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: "esnext",
  },
});
