import { defineConfig } from "tsup";

export default defineConfig([
  // ESM build
  {
    entry: ["src/index.ts"],
    format: ["esm"],
    clean: true,
    sourcemap: true,
    outDir: "dist/esm",
    target: "es2022",
    esbuildOptions(options) {
      options.platform = "node";
    }
  },
  // CJS build
  {
    entry: ["src/index.ts"],
    format: ["cjs"],
    sourcemap: true,
    outDir: "dist/cjs",
    target: "es2022",
    esbuildOptions(options) {
      options.platform = "node";
    },
    cjsExtension: ".cjs"
  },
  // DTS build for both ESM and CJS
  {
    entry: ["src/index.ts"],
    dts: true,
    sourcemap: false,
    outDir: "dist/types",
    clean: false
  }
]);
