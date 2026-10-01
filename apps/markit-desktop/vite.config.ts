import { defineConfig } from "vite";
import { resolve } from "node:path";

// Renderer build only. The Electron main process and preload are built by
// scripts/build-main.mjs (esbuild); tests run via vitest.config.ts.
export default defineConfig({
  root: "src/renderer",
  base: "./",
  build: {
    outDir: resolve(import.meta.dirname, "dist/renderer"),
    emptyOutDir: true,
  },
});
