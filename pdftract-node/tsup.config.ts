import { defineConfig } from "tsup";

export default defineConfig([
  // ESM build (no types - types built separately)
  {
    entry: ["src/index.ts"],
    format: ["esm"],
    clean: true,
    sourcemap: true,
    outDir: "dist/esm",
    target: "es2022",
    dts: false,
    esbuildOptions(options) {
      options.platform = "node";
    }
  },
  // CJS build (no types - types built separately)
  {
    entry: ["src/index.ts"],
    format: ["cjs"],
    sourcemap: true,
    outDir: "dist/cjs",
    target: "es2022",
    dts: false,
    esbuildOptions(options) {
      options.platform = "node";
    },
    cjsExtension: ".cjs"
  },
  // Types only - shared for both ESM and CJS (using .d.ts extension)
  {
    entry: ["src/index.ts"],
    format: ["esm"],
    dts: { only: true },
    outDir: "dist/types",
    clean: false,
    esbuildOptions(options) {
      options.platform = "neutral";
    }
  }
]);
