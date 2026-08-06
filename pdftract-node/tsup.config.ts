import { defineConfig } from 'tsup';

export default defineConfig([
  {
    entry: ['src/index.ts'],
    format: 'esm',
    dts: false,
    clean: true,
    sourcemap: true,
    target: 'es2022',
    outDir: 'dist/esm',
    splitting: false,
    esbuildOptions(options) {
      options.platform = 'node';
    },
  },
  {
    entry: ['src/index.ts'],
    format: 'cjs',
    dts: false,
    clean: false,
    sourcemap: true,
    target: 'es2022',
    outDir: 'dist/cjs',
    splitting: false,
    esbuildOptions(options) {
      options.platform = 'node';
    },
  },
  {
    entry: ['src/index.ts'],
    dts: { only: true },
    clean: false,
    outDir: 'dist/types',
  },
]);
