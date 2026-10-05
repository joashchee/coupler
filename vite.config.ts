import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// Read once at build time, not import()'d, so this stays a plain build-time
// constant (see __APP_VERSION__ below) rather than bundling package.json
// itself into the app.
const pkg = JSON.parse(
  readFileSync(fileURLToPath(new URL("./package.json", import.meta.url)), "utf-8"),
);

// The web build (`--mode web`): the same app with
// Tauri's modules swapped for the page's stand-ins in src/web/, built to
// dist-web/. scripts/web-build.sh builds the WebAssembly core first.
const webAliases = {
  "@tauri-apps/api/core": fileURLToPath(new URL("./src/web/core.ts", import.meta.url)),
  "@tauri-apps/api/event": fileURLToPath(new URL("./src/web/event.ts", import.meta.url)),
  "@tauri-apps/api/webview": fileURLToPath(new URL("./src/web/webview.ts", import.meta.url)),
  "@tauri-apps/plugin-dialog": fileURLToPath(new URL("./src/web/dialog.ts", import.meta.url)),
};

// https://vite.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [react()],
  resolve: mode === "web" ? { alias: webAliases } : undefined,
  build: mode === "web" ? { outDir: "dist-web", emptyOutDir: true } : undefined,
  define: {
    // package.json's version is the single source of truth for the
    // displayed app version — see CLAUDE.md's version-bump checklist.
    __APP_VERSION__: JSON.stringify(pkg.version),
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    // The web build's own dev server sits beside the desktop's.
    port: mode === "web" ? 1462 : 1460,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1461,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
