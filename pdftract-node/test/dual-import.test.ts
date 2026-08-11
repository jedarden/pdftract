/**
 * Dual ESM/CJS import tests for pdftract Node.js SDK
 *
 * This test verifies that the SDK can be imported both as ESM and CJS,
 * ensuring compatibility with modern bundlers (Vite) and legacy toolchains (Webpack 4).
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { readFileSync, unlinkSync, writeFileSync, mkdirSync, rmSync } from 'fs';
import { join, dirname } from 'path';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

describe('Dual ESM/CJS Imports', () => {
  const tempDir = join(process.env.TMPDIR || '/tmp', 'pdftract-import-test');
  const packageRoot = join(__dirname, '..');

  beforeAll(() => {
    // Create temp directory
    try {
      mkdirSync(tempDir, { recursive: true });
    } catch (e) {
      // Directory already exists
    }

    // Copy package.json to temp directory for proper resolution
    const pkgJson = JSON.parse(readFileSync(join(packageRoot, 'package.json'), 'utf-8'));
    pkgJson.dependencies = {
      '@pdftract/sdk': `file://${packageRoot}`
    };
    writeFileSync(join(tempDir, 'package.json'), JSON.stringify(pkgJson, null, 2));
  });

  afterAll(() => {
    // Cleanup temp directory
    try {
      rmSync(tempDir, { recursive: true, force: true });
    } catch (e) {
      // Ignore cleanup errors
    }
  });

  it('should support ESM import syntax from built dist', { timeout: 30000 }, () => {
    const testCode = `
import { Client, path, bytes, url } from '${join(packageRoot, 'dist/esm/index.js')}';

const client = new Client();
console.log('ESM import successful');
console.log('Client type:', typeof client);
console.log('Path function type:', typeof path);
console.log('Bytes function type:', typeof bytes);
console.log('URL function type:', typeof url);

// Test that exports are functions/classes
console.log('Client is class:', typeof Client === 'function');
console.log('path is function:', typeof path === 'function');
console.log('bytes is function:', typeof bytes === 'function');
console.log('url is function:', typeof url === 'function');
`;

    const result = execSync(`node --input-type=module -e "${testCode}"`, {
      cwd: packageRoot,
      stdio: 'pipe',
      timeout: 10000
    }).toString();

    expect(result).toContain('ESM import successful');
    expect(result).toContain('Client is class: true');
    expect(result).toContain('path is function: true');
    expect(result).toContain('bytes is function: true');
    expect(result).toContain('url is function: true');
  });

  it('should support CJS require syntax from built dist', { timeout: 30000 }, () => {
    const testCode = `
const { Client, path, bytes, url } = require('${join(packageRoot, 'dist/cjs/index.cjs')}');

const client = new Client();
console.log('CJS require successful');
console.log('Client type:', typeof client);
console.log('Path function type:', typeof path);
console.log('Bytes function type:', typeof bytes);
console.log('URL function type:', typeof url);

// Test that exports are functions/classes
console.log('Client is class:', typeof Client === 'function');
console.log('path is function:', typeof path === 'function');
console.log('bytes is function:', typeof bytes === 'function');
console.log('url is function:', typeof url === 'function');
`;

    const result = execSync(`node -e "${testCode}"`, {
      cwd: packageRoot,
      stdio: 'pipe',
      timeout: 10000
    }).toString();

    expect(result).toContain('CJS require successful');
    expect(result).toContain('Client is class: true');
    expect(result).toContain('path is function: true');
    expect(result).toContain('bytes is function: true');
    expect(result).toContain('url is function: true');
  });

  it('should export error classes in both ESM and CJS', { timeout: 30000 }, () => {
    const esmTest = `
import {
  PdftractError,
  CorruptPdfError,
  EncryptionError,
  SourceUnreachableError,
  RemoteFetchInterruptedError,
  TlsError,
  ReceiptVerifyError,
  ValidationError
} from '${join(packageRoot, 'dist/esm/index.js')}';

console.log('PdftractError:', typeof PdftractError);
console.log('CorruptPdfError:', typeof CorruptPdfError);
console.log('EncryptionError:', typeof EncryptionError);
console.log('SourceUnreachableError:', typeof SourceUnreachableError);
console.log('RemoteFetchInterruptedError:', typeof RemoteFetchInterruptedError);
console.log('TlsError:', typeof TlsError);
console.log('ReceiptVerifyError:', typeof ReceiptVerifyError);
console.log('ValidationError:', typeof ValidationError);

// Verify all are functions (class constructors)
console.log('All error classes exported:',
  typeof PdftractError === 'function' &&
  typeof CorruptPdfError === 'function' &&
  typeof EncryptionError === 'function' &&
  typeof SourceUnreachableError === 'function' &&
  typeof RemoteFetchInterruptedError === 'function' &&
  typeof TlsError === 'function' &&
  typeof ReceiptVerifyError === 'function' &&
  typeof ValidationError === 'function'
);
`;

    const cjsTest = `
const {
  PdftractError,
  CorruptPdfError,
  EncryptionError,
  SourceUnreachableError,
  RemoteFetchInterruptedError,
  TlsError,
  ReceiptVerifyError,
  ValidationError
} = require('${join(packageRoot, 'dist/cjs/index.cjs')}');

console.log('PdftractError:', typeof PdftractError);
console.log('CorruptPdfError:', typeof CorruptPdfError);
console.log('EncryptionError:', typeof EncryptionError);
console.log('SourceUnreachableError:', typeof SourceUnreachableError);
console.log('RemoteFetchInterruptedError:', typeof RemoteFetchInterruptedError);
console.log('TlsError:', typeof TlsError);
console.log('ReceiptVerifyError:', typeof ReceiptVerifyError);
console.log('ValidationError:', typeof ValidationError);

// Verify all are functions (class constructors)
console.log('All error classes exported:',
  typeof PdftractError === 'function' &&
  typeof CorruptPdfError === 'function' &&
  typeof EncryptionError === 'function' &&
  typeof SourceUnreachableError === 'function' &&
  typeof RemoteFetchInterruptedError === 'function' &&
  typeof TlsError === 'function' &&
  typeof ReceiptVerifyError === 'function' &&
  typeof ValidationError === 'function'
);
`;

    const esmResult = execSync(`node --input-type=module -e "${esmTest}"`, {
      cwd: packageRoot,
      stdio: 'pipe',
      timeout: 10000
    }).toString();

    expect(esmResult).toContain('All error classes exported: true');

    const cjsResult = execSync(`node -e "${cjsTest}"`, {
      cwd: packageRoot,
      stdio: 'pipe',
      timeout: 10000
    }).toString();

    expect(cjsResult).toContain('All error classes exported: true');
  });

  it('should export utility functions in both ESM and CJS', { timeout: 30000 }, () => {
    const esmTest = `
import {
  normalizeOptions,
  coerceSource,
  validateOptions,
  mergeOptions,
  lazyOptions,
  isUrl,
  isBuffer
} from '${join(packageRoot, 'dist/esm/index.js')}';

console.log('normalizeOptions:', typeof normalizeOptions);
console.log('coerceSource:', typeof coerceSource);
console.log('validateOptions:', typeof validateOptions);
console.log('mergeOptions:', typeof mergeOptions);
console.log('lazyOptions:', typeof lazyOptions);
console.log('isUrl:', typeof isUrl);
console.log('isBuffer:', typeof isBuffer);

// Verify all are functions
console.log('All utilities exported:',
  typeof normalizeOptions === 'function' &&
  typeof coerceSource === 'function' &&
  typeof validateOptions === 'function' &&
  typeof mergeOptions === 'function' &&
  typeof lazyOptions === 'function' &&
  typeof isUrl === 'function' &&
  typeof isBuffer === 'function'
);
`;

    const cjsTest = `
const {
  normalizeOptions,
  coerceSource,
  validateOptions,
  mergeOptions,
  lazyOptions,
  isUrl,
  isBuffer
} = require('${join(packageRoot, 'dist/cjs/index.cjs')}');

console.log('normalizeOptions:', typeof normalizeOptions);
console.log('coerceSource:', typeof coerceSource);
console.log('validateOptions:', typeof validateOptions);
console.log('mergeOptions:', typeof mergeOptions);
console.log('lazyOptions:', typeof lazyOptions);
console.log('isUrl:', typeof isUrl);
console.log('isBuffer:', typeof isBuffer);

// Verify all are functions
console.log('All utilities exported:',
  typeof normalizeOptions === 'function' &&
  typeof coerceSource === 'function' &&
  typeof validateOptions === 'function' &&
  typeof mergeOptions === 'function' &&
  typeof lazyOptions === 'function' &&
  typeof isUrl === 'function' &&
  typeof isBuffer === 'function'
);
`;

    const esmResult = execSync(`node --input-type=module -e "${esmTest}"`, {
      cwd: packageRoot,
      stdio: 'pipe',
      timeout: 10000
    }).toString();

    expect(esmResult).toContain('All utilities exported: true');

    const cjsResult = execSync(`node -e "${cjsTest}"`, {
      cwd: packageRoot,
      stdio: 'pipe',
      timeout: 10000
    }).toString();

    expect(cjsResult).toContain('All utilities exported: true');
  });
});
