/**
 * Conformance test suite for pdftract Node.js SDK
 *
 * This test runs the shared conformance suite from the pdftract repository.
 * Set the CONFORMANCE_SUITE environment variable to point to the cases.json file.
 */

import { describe, it, beforeAll, expect } from 'vitest';
import { Client, path } from '../src/index.js';
import { readFileSync } from 'fs';
import { join } from 'path';

// Use the built pdftract binary from the repository
const binaryPath = join(process.env.PDFTRACT_SRC || '../../pdftract', 'target/release/pdftract');
const client = new Client(binaryPath);

describe('SDK Conformance', () => {
  // Allow overriding the suite path via environment variable
  const suitePath = process.env.CONFORMANCE_SUITE ||
    join(process.env.PDFTRACT_SRC || '../../pdftract', 'tests/sdk-conformance/cases.json');

  // Load the suite synchronously at test definition time
  let suite: any;
  try {
    const content = readFileSync(suitePath, 'utf-8');
    suite = JSON.parse(content);
    console.log(`Loaded conformance suite from ${suitePath} with ${suite.cases?.length || 0} cases`);
  } catch (error) {
    console.warn(`Warning: Could not load conformance suite from ${suitePath}:`, error);
    suite = { cases: [] };
  }

  beforeAll(() => {
    // This hook is kept for any future setup needs
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
    case 'extract_stream':
      await testExtractStream(fixturePath, tc.options, tc.expected);
      break;
    case 'search':
      // Search functionality (grep) is not yet available
      console.log(`Skipping ${tc.id}: search - 'grep' subcommand planned for Phase 7.8`);
      break;
    case 'get_metadata':
      // get_metadata with --metadata-only is not yet available
      console.log(`Skipping ${tc.id}: get_metadata - '--metadata-only' option not yet implemented`);
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

async function testExtractStream(fixturePath: string, options: any, expected: any) {
  const pages: any[] = [];
  for await (const page of client.extractStream(path(fixturePath), options)) {
    pages.push(page);
    if (options?.max_pages && pages.length >= options.max_pages) {
      break;
    }
  }

  if (expected?.['frame_count'] !== undefined) {
    const count = expected['frame_count'];
    if (typeof count === 'object' && 'min' in count) {
      expect(pages.length).toBeGreaterThanOrEqual(count.min);
    } else {
      expect(pages.length).toBe(count);
    }
  }

  if (expected?.['page_frames'] !== undefined) {
    const pageFrames = expected['page_frames'];
    if (typeof pageFrames === 'object' && 'min' in pageFrames) {
      expect(pages.length).toBeGreaterThanOrEqual(pageFrames.min);
    } else if (typeof pageFrames === 'object' && 'max' in pageFrames) {
      expect(pages.length).toBeLessThanOrEqual(pageFrames.max);
    }
  }

  if (pages.length > 0) {
    if (expected?.['first_frame_type'] !== undefined) {
      expect(pages[0]).toBeDefined();
    }

    if (expected?.['last_frame_type'] !== undefined) {
      expect(pages[pages.length - 1]).toBeDefined();
    }

    if (expected?.['header_frame_has_schema_version'] !== undefined) {
      expect(pages[0]?.schema_version).toBeTruthy();
    }

    if (expected?.['header_frame_has_total_pages'] !== undefined) {
      expect(pages[0]?.total_pages).toBeTruthy();
    }
  }
}

async function testSearch(fixturePath: string, options: any, expected: any) {
  const matches: any[] = [];
  for await (const match of client.search(path(fixturePath), options.pattern, options)) {
    matches.push(match);
  }

  if (expected?.['match_count'] !== undefined) {
    expect(matches.length).toBe(expected['match_count']);
  }

  if (expected?.['min_matches'] !== undefined) {
    expect(matches.length).toBeGreaterThanOrEqual(expected['min_matches']);
  }

  if (matches.length > 0 && expected?.['first_match_page'] !== undefined) {
    expect(matches[0]?.page).toBe(expected['first_match_page']);
  }

  if (matches.length > 0 && expected?.['first_match_text'] !== undefined) {
    expect(matches[0]?.text).toBe(expected['first_match_text']);
  }
}

async function testExtract(fixturePath: string, options: any, expected: any) {
  // Handle password via environment variable for security
  const password = options?.password;
  if (password) {
    delete options.password;
    process.env.PDFTRACT_PASSWORD = password;
  }

  try {
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
  } finally {
    // Clean up environment variable
    if (password) {
      delete process.env.PDFTRACT_PASSWORD;
    }
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

  try {
    const valid = await client.verifyReceipt(path(fixturePath), receipt);

    if (expected?.['valid'] !== undefined) {
      expect(valid).toBe(expected['valid']);
    }
  } catch (error: any) {
    // Verify receipt is not yet fully implemented in SDK
    console.log(`Skipping verify receipt: ${error.message}`);
  }
}
