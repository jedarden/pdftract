#!/usr/bin/env node
/**
 * CJS Require Test for @pdftract/sdk
 *
 * This script verifies that the SDK can be imported using CommonJS require.
 * Run with: node test/cjs-import.test.cjs
 */

const { Client, path, url, bytes } = require('../dist/cjs/index.cjs');
const {
  PdftractError,
  CorruptPdfError,
  EncryptionError,
  SourceUnreachableError,
  RemoteFetchInterruptedError,
  TlsError,
  ReceiptVerifyError,
  ValidationError
} = require('../dist/cjs/index.cjs');

console.log('✅ CJS require successful');
console.log('✅ Client class required:', typeof Client);
console.log('✅ Helper functions required:', { path: typeof path, url: typeof url, bytes: typeof bytes });
console.log('✅ Error classes required:', {
  PdftractError: typeof PdftractError,
  CorruptPdfError: typeof CorruptPdfError,
  EncryptionError: typeof EncryptionError,
  SourceUnreachableError: typeof SourceUnreachableError,
  RemoteFetchInterruptedError: typeof RemoteFetchInterruptedError,
  TlsError: typeof TlsError,
  ReceiptVerifyError: typeof ReceiptVerifyError,
  ValidationError: typeof ValidationError
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

console.log('\n🎉 CJS require test PASSED');
