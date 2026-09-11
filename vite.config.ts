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
  build: {
    target: "es2022",
    rollupOptions: {
      output: {
        /* Keep the first paint from carrying every secondary page and icon
           implementation in one monolith. These are stable product seams,
           not arbitrary module-count buckets, so a page can be inspected or
           replaced without invalidating the rest of the cache. */
        manualChunks: {
          "vendor-react": ["react", "react-dom"],
          "vendor-icons": ["lucide-react"],
          "page-settings": ["./src/components/SettingsPage.tsx"],
          "page-history": ["./src/components/HistoryPage.tsx"],
          "page-privacy": ["./src/components/PrivacyPage.tsx"],
          "page-about": ["./src/components/AboutPage.tsx"],
        },
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
  },
});
