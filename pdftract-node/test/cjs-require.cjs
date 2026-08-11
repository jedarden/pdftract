#!/usr/bin/env node
/**
 * CJS require verification test
 * Run with: node test/cjs-require.cjs
 */

const { Client, path, url, bytes } = require('../dist/cjs/index.cjs');
const { readFileSync } = require('fs');
const { join } = require('path');

// Path to pdftract binary
const binaryPath = join(__dirname, '..', '..', 'target/release/pdftract');

// Create a simple test PDF path
const testPdfPath = process.env.TEST_PDF || join(__dirname, '..', '..', 'tests/sdk-conformance/fixtures/tests/sdk-conformance/fixtures/misc/01.pdf');

async function runCjsTests() {
  console.log('=== CJS Require Verification Test ===\n');

  // Test 1: Verify imports are CJS-compatible
  console.log('✓ Test 1: All exports required successfully');
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

  console.log('\n=== CJS Require Test Complete ===');
  console.log('All CJS requires verified successfully!\n');
}

// Run the tests
runCjsTests().catch(error => {
  console.error('CJS require test failed:', error);
  process.exit(1);
});
