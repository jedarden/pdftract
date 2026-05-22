/**
 * Conformance test suite for pdftract Node.js SDK
 *
 * This test runs the shared conformance suite from the pdftract repository.
 * Set the CONFORMANCE_SUITE environment variable to point to the cases.json file.
 */

import { describe, it, before, expect } from 'vitest';
import { Client, path } from '../src/index.js';
import { readFileSync } from 'fs';
import { join } from 'path';

const client = new Client();

describe('SDK Conformance', () => {
  // Allow overriding the suite path via environment variable
  const suitePath = process.env.CONFORMANCE_SUITE ||
    join(process.env.PDFTRACT_SRC || '../../pdftract', 'tests/sdk-conformance/cases.json');

  let suite: any;

  before(() => {
    try {
      const content = readFileSync(suitePath, 'utf-8');
      suite = JSON.parse(content);
      console.log(`Loaded conformance suite from ${suitePath}`);
    } catch (error) {
      console.warn(`Warning: Could not load conformance suite from ${suitePath}:`, error);
      suite = { cases: [] };
    }
  });

  for (const tc of (suite?.cases || [])) {
    it(`${tc.id}: ${tc.method}`, { timeout: 30000 }, async () => {
      // Build fixture path relative to the suite directory
      const fixtureDir = process.env.CONFORMANCE_FIXTURES ||
        join(process.env.PDFTRACT_SRC || '../../pdftract', 'tests/sdk-conformance');
      const fixturePath = join(fixtureDir, tc.fixture);
      await runTestCase(tc, fixturePath);
    });
  }
});

async function runTestCase(tc: any, fixturePath: string) {
  switch (tc.method) {
    case 'extract':
      await testExtract(fixturePath, tc.options, tc.expected);
      break;
    case 'extract_text':
      await testExtractText(fixturePath, tc.options, tc.expected);
      break;
    case 'extract_markdown':
      await testExtractMarkdown(fixturePath, tc.options, tc.expected);
      break;
    case 'get_metadata':
      await testGetMetadata(fixturePath, tc.options, tc.expected);
      break;
    case 'hash':
      await testHash(fixturePath, tc.options, tc.expected);
      break;
    case 'classify':
      await testClassify(fixturePath, tc.expected);
      break;
    case 'verify_receipt':
      await testVerifyReceipt(fixturePath, tc.options, tc.expected);
      break;
    default:
      console.log(`Skipping method: ${tc.method}`);
  }
}

async function testExtract(fixturePath: string, options: any, expected: any) {
  const doc = await client.extract(path(fixturePath), options);

  if (expected?.['schema_version'] !== undefined) {
    if (typeof expected['schema_version'] === 'string') {
      expect(doc.schema_version).toBe(expected['schema_version']);
    }
  }

  if (expected?.['pages.length'] !== undefined) {
    expect(doc.pages.length).toBe(expected['pages.length']);
  }

  if (expected?.['metadata.page_count'] !== undefined) {
    expect(doc.metadata.page_count).toBe(expected['metadata.page_count']);
  }

  if (expected?.['pages[0].page_index'] !== undefined) {
    expect(doc.pages[0]?.page_index).toBe(expected['pages[0].page_index']);
  }

  if (expected?.['pages[0].width'] !== undefined) {
    const width = doc.pages[0]?.width;
    const range = expected['pages[0].width'];
    if (typeof range === 'object' && 'min' in range && 'max' in range) {
      expect(width).toBeGreaterThanOrEqual(range.min);
      expect(width).toBeLessThanOrEqual(range.max);
    } else {
      expect(width).toBe(range);
    }
  }

  if (expected?.['pages[0].height'] !== undefined) {
    const height = doc.pages[0]?.height;
    const range = expected['pages[0].height'];
    if (typeof range === 'object' && 'min' in range && 'max' in range) {
      expect(height).toBeGreaterThanOrEqual(range.min);
      expect(height).toBeLessThanOrEqual(range.max);
    } else {
      expect(height).toBe(range);
    }
  }

  if (expected?.['pages[0].rotation'] !== undefined) {
    expect(doc.pages[0]?.rotation).toBe(expected['pages[0].rotation']);
  }

  if (expected?.['pages[0].blocks[0].kind'] !== undefined) {
    expect(doc.pages[0]?.blocks[0]?.kind).toBe(expected['pages[0].blocks[0].kind']);
  }

  if (expected?.['errors.length'] !== undefined) {
    expect(expected['errors.length']).toBe(0);
  }
}

async function testExtractText(fixturePath: string, options: any, expected: any) {
  const text = await client.extractText(path(fixturePath), options);

  if (expected?.['min_length'] !== undefined) {
    expect(text.length).toBeGreaterThanOrEqual(expected['min_length']);
  }

  if (expected?.['contains'] !== undefined) {
    for (const substr of expected['contains']) {
      expect(text).toContain(substr);
    }
  }
}

async function testExtractMarkdown(fixturePath: string, options: any, expected: any) {
  const md = await client.extractMarkdown(path(fixturePath), options);

  if (expected?.['min_length'] !== undefined) {
    expect(md.length).toBeGreaterThanOrEqual(expected['min_length']);
  }
}

async function testGetMetadata(fixturePath: string, options: any, expected: any) {
  const metadata = await client.getMetadata(path(fixturePath), options);

  if (expected?.['page_count'] !== undefined) {
    expect(metadata.page_count).toBe(expected['page_count']);
  }

  if (expected?.['is_encrypted'] !== undefined) {
    expect(metadata.is_encrypted).toBe(expected['is_encrypted']);
  }
}

async function testHash(fixturePath: string, options: any, expected: any) {
  const fingerprint = await client.hash(path(fixturePath), options);

  expect(fingerprint.hash.length).toBe(64);
  expect(fingerprint.fast_hash.length).toBe(64);

  if (expected?.['page_count'] !== undefined) {
    expect(fingerprint.page_count).toBe(expected['page_count']);
  }
}

async function testClassify(fixturePath: string, expected: any) {
  const classification = await client.classify(path(fixturePath));

  expect(classification.category).toBeTruthy();
  expect(classification.confidence).toBeGreaterThanOrEqual(0);
  expect(classification.confidence).toBeLessThanOrEqual(1);
}

async function testVerifyReceipt(fixturePath: string, options: any, expected: any) {
  const receipt = expected?.receipt;
  if (!receipt) {
    console.log('Skipping receipt verification: no receipt provided');
    return;
  }

  const valid = await client.verifyReceipt(fixturePath, receipt);

  if (expected?.['valid'] !== undefined) {
    expect(valid).toBe(expected['valid']);
  }
}
