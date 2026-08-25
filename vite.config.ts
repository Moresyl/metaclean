import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  // No `cssTarget`: Tailwind v4 leans on `color-mix()`, `@property` and nesting,
  // and lowering the output past Chrome 111 would strip the tokens the theme is
  // built out of. WebView2 ships evergreen, so the floor is never the one here.
  build: { target: "es2022" },
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
  },
});
