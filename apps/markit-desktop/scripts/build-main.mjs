// Build the Electron main process and the (CJS) sandboxed preload with
// esbuild. The renderer is built by `vite build`.
import { build } from "esbuild";
import { mkdirSync } from "node:fs";

mkdirSync("dist/main", { recursive: true });

await build({
  entryPoints: ["src/main/main.ts"],
  bundle: true,
  platform: "node",
  format: "esm",
  outfile: "dist/main/main.js",
  external: ["electron", "node:*"],
  sourcemap: "inline",
});

await build({
  entryPoints: ["src/preload/preload.ts"],
  bundle: true,
  platform: "node",
  format: "cjs",
  outfile: "dist/main/preload.cjs",
  external: ["electron"],
  sourcemap: "inline",
});

console.log("main + preload built");
