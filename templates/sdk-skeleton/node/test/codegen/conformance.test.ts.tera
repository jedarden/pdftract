/**
 * Conformance test suite for pdftract Node.js SDK
 * Auto-generated - do not edit manually
 */

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert';
import { Client, path } from '../../src/index.js';
import { readFileSync } from 'fs';
import { join } from 'path';

const client = new Client();

describe('SDK Conformance', () => {
  const suitePath = process.env.CONFORMANCE_SUITE || 'tests/sdk-conformance/cases.json';

  let suite: any;

  before(() => {
    try {
      const content = readFileSync(suitePath, 'utf-8');
      suite = JSON.parse(content);
    } catch (error) {
      console.warn(`Warning: Could not load conformance suite from ${suitePath}`);
      suite = { cases: [] };
    }
  });

  for (const tc of (suite?.cases || [])) {
    it(`${tc.id}: ${tc.method}`, { timeout: 30000 }, async () => {
      const fixturePath = join('fixtures', tc.fixture);
      await runTestCase(tc, fixturePath);
    });
  }
});

async function runTestCase(tc: any, fixturePath: string) {
  switch (tc.method) {
    case 'extract':
      await testExtract(fixturePath, tc.options, tc.assertions);
      break;
    case 'extract_text':
      await testExtractText(fixturePath, tc.options, tc.assertions);
      break;
    case 'extract_markdown':
      await testExtractMarkdown(fixturePath, tc.options, tc.assertions);
      break;
    case 'get_metadata':
      await testGetMetadata(fixturePath, tc.options, tc.assertions);
      break;
    case 'hash':
      await testHash(fixturePath, tc.options, tc.assertions);
      break;
    case 'classify':
      await testClassify(fixturePath, tc.assertions);
      break;
    case 'verify_receipt':
      await testVerifyReceipt(fixturePath, tc.options, tc.assertions);
      break;
    default:
      console.log(`Skipping method: ${tc.method}`);
  }
}

async function testExtract(fixturePath: string, options: any, assertions: any) {
  const doc = await client.extract(path(fixturePath), options);

  if (assertions?.page_count !== undefined) {
    assert.strictEqual(doc.pages.length, assertions.page_count);
  }

  if (assertions?.has_title) {
    assert.ok(doc.metadata.title);
  }

  if (assertions?.has_blocks) {
    const hasBlocks = doc.pages.some((p: any) => p.blocks && p.blocks.length > 0);
    assert.ok(hasBlocks);
  }
}

async function testExtractText(fixturePath: string, options: any, assertions: any) {
  const text = await client.extractText(path(fixturePath), options);

  if (assertions?.min_length !== undefined) {
    assert.ok(text.length >= assertions.min_length);
  }

  if (assertions?.contains) {
    for (const substr of assertions.contains) {
      assert.ok(text.includes(substr), `Expected text to contain: ${substr}`);
    }
  }
}

async function testExtractMarkdown(fixturePath: string, options: any, assertions: any) {
  const md = await client.extractMarkdown(path(fixturePath), options);

  if (assertions?.min_length !== undefined) {
    assert.ok(md.length >= assertions.min_length);
  }
}

async function testGetMetadata(fixturePath: string, options: any, assertions: any) {
  const metadata = await client.getMetadata(path(fixturePath), options);

  if (assertions?.page_count !== undefined) {
    assert.strictEqual(metadata.page_count, assertions.page_count);
  }
}

async function testHash(fixturePath: string, options: any, assertions: any) {
  const fingerprint = await client.hash(path(fixturePath), options);

  assert.strictEqual(fingerprint.hash.length, 64);
  assert.strictEqual(fingerprint.fast_hash.length, 64);

  if (assertions?.page_count !== undefined) {
    assert.strictEqual(fingerprint.page_count, assertions.page_count);
  }
}

async function testClassify(fixturePath: string, assertions: any) {
  const classification = await client.classify(path(fixturePath));

  assert.ok(classification.category);
  assert.ok(classification.confidence >= 0 && classification.confidence <= 1);
}

async function testVerifyReceipt(fixturePath: string, options: any, assertions: any) {
  const receipt = assertions?.receipt;
  if (!receipt) {
    console.log('Skipping receipt verification: no receipt provided');
    return;
  }

  const valid = await client.verifyReceipt(fixturePath, receipt);

  if (assertions?.valid !== undefined) {
    assert.strictEqual(valid, assertions.valid);
  }
}
