#!/usr/bin/env node
/**
 * ESM Import Test for @pdftract/sdk
 *
 * This script verifies that the SDK can be imported using ES6 import syntax.
 * Run with: node test/esm-import.test.mjs
 */

import { Client, path, url, bytes } from '../dist/esm/index.js';
import { PdftractError, CorruptPdfError, EncryptionError } from '../dist/esm/index.js';

console.log('✅ ESM import successful');
console.log('✅ Client class imported:', typeof Client);
console.log('✅ Helper functions imported:', { path: typeof path, url: typeof url, bytes: typeof bytes });
console.log('✅ Error classes imported:', {
  PdftractError: typeof PdftractError,
  CorruptPdfError: typeof CorruptPdfError,
  EncryptionError: typeof EncryptionError
});

// Test that we can instantiate the Client
const client = new Client('pdftract');
console.log('✅ Client instantiated:', client.constructor.name);

// Test that the Client has all 9 methods
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

const missingMethods = methods.filter(m => typeof client[m] !== 'function');
if (missingMethods.length > 0) {
  console.error('❌ Missing methods:', missingMethods);
  process.exit(1);
} else {
  console.log('✅ All 9 SDK methods present');
}

console.log('\n🎉 ESM import test PASSED');
