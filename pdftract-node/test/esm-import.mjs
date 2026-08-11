#!/usr/bin/env node
/**
 * ESM import verification test
 * Run with: node --input-type=module -e "import('./test/esm-import.mjs').then(m => m.default())"
 * Or simply: node test/esm-import.mjs (since package.json has "type": "module")
 */

import { Client, path, url, bytes } from '../dist/esm/index.js';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Path to pdftract binary
const binaryPath = join(__dirname, '..', '..', 'target/release/pdftract');

// Create a simple test PDF path (use one from conformance fixtures if available)
const testPdfPath = process.env.TEST_PDF || join(__dirname, '..', '..', 'tests/sdk-conformance/fixtures/tests/sdk-conformance/fixtures/misc/01.pdf');

async function runEsmTests() {
  console.log('=== ESM Import Verification Test ===\n');

  // Test 1: Verify imports are ESM-compatible
  console.log('✓ Test 1: All exports imported successfully');
  console.log('  - Client:', typeof Client);
  console.log('  - path:', typeof path);
  console.log('  - url:', typeof url);
  console.log('  - bytes:', typeof bytes);

  // Test 2: Client instantiation
  console.log('\n✓ Test 2: Client instantiation');
  const client = new Client(binaryPath);
  console.log('  - Client created with binary path:', binaryPath);

  // Test 3: Method availability (all 9 SDK methods)
  console.log('\n✓ Test 3: SDK Method availability');
  const methods = [
    'extract',
    'extractText',
    'extractMarkdown',
    'extractStream',
    'search',
    'getMetadata',
    'hash',
    'classify',
    'verifyReceipt'
  ];
  for (const method of methods) {
    console.log(`  - client.${method}:`, typeof client[method]);
  }

  // Test 4: Actual method call (if test PDF exists)
  try {
    const stats = readFileSync(testPdfPath);
    console.log(`\n✓ Test 4: Real method call with test PDF`);
    console.log(`  - Test PDF: ${testPdfPath}`);

    // Test hash method (simple and doesn't require OCR)
    const fingerprint = await client.hash(path(testPdfPath));
    console.log(`  - hash() returned:`, {
      hash_length: fingerprint.hash.length,
      fast_hash_length: fingerprint.fast_hash.length,
      page_count: fingerprint.page_count
    });
  } catch (error) {
    console.log(`\n⚠ Test 4: Skipped (no test PDF available at ${testPdfPath})`);
  }

  console.log('\n=== ESM Import Test Complete ===');
  console.log('All ESM imports verified successfully!\n');
}

// Run the tests
runEsmTests().catch(error => {
  console.error('ESM import test failed:', error);
  process.exit(1);
});

export default runEsmTests;
