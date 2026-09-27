import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  build: {
    rollupOptions: {
      output: {
        // Desktop app: assets load from local disk, not a network, so this
        // is about parallel parsing and cache reuse across rebuilds rather
        // than load latency. Splitting the large, rarely-changing terminal
        // library out of the app bundle means editing app code doesn't
        // invalidate xterm's cache entry.
        manualChunks(id: string) {
          if (id.includes("@xterm")) return "xterm";
          if (id.includes("node_modules/react") || id.includes("node_modules/react-dom")) {
            return "vendor";
          }
        },
      },
    },
  },
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
