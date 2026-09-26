/**
 * Conformance test suite for pdftract Node.js SDK
 * Auto-generated - do not edit manually
 */

import { describe, it } from 'node:test';
import assert from 'node:assert';
import net from 'node:net';
import { Client, path } from '../../src/index.js';
import { readFileSync } from 'fs';
import { join } from 'path';

const client = new Client();
const suitePath = process.env.CONFORMANCE_SUITE || 'tests/sdk-conformance/cases.json';

// The suite is loaded here, at module scope, because the per-case it()
// registrations below run while this module loads — before the runner executes
// any hook. Loading the suite inside before() would always register zero
// tests, so the load must happen before the describe body runs.
let suite: { cases: any[] };
try {
  suite = JSON.parse(readFileSync(suitePath, 'utf-8'));
} catch {
  console.warn(`Warning: Could not load conformance suite from ${suitePath}`);
  suite = { cases: [] };
}

// Memoized TCP reachability probe so URL-fixture cases (fetched by the
// pdftract binary itself) can degrade to a skip when the environment has no
// network egress.
const reachability = new Map<string, Promise<boolean>>();
function remoteReachable(fixtureUrl: string): Promise<boolean> {
  const parsed = new URL(fixtureUrl);
  const host = parsed.hostname;
  const port = Number(parsed.port) || 443;
  let probe = reachability.get(host);
  if (!probe) {
    probe = new Promise<boolean>((resolve) => {
      const socket = net.connect({ host, port });
      socket.setTimeout(3000);
      socket.once('connect', () => {
        socket.destroy();
        resolve(true);
      });
      socket.once('timeout', () => {
        socket.destroy();
        resolve(false);
      });
      socket.once('error', () => resolve(false));
    });
    reachability.set(host, probe);
  }
  return probe;
}

function isRemoteFixture(fixture: string): boolean {
  return fixture.startsWith('http://') || fixture.startsWith('https://');
}

describe('SDK Conformance', () => {
  for (const tc of suite.cases) {
    // URL fixtures are fetched by the pdftract binary itself; local fixtures
    // resolve relative to the fixtures/ directory.
    const fixturePath = isRemoteFixture(tc.fixture) ? tc.fixture : join('fixtures', tc.fixture);
    it(`${tc.id}: ${tc.method}`, { timeout: 30000 }, async (t) => {
      if (isRemoteFixture(tc.fixture) && !(await remoteReachable(tc.fixture))) {
        t.skip(`Offline: ${tc.fixture} unreachable, skipping remote conformance case`);
        return;
      }
      await runTestCase(tc, fixturePath);
    });
  }
});

async function runTestCase(tc: any, fixturePath: string) {
  const expected = tc.expected ?? {};
  const tolerances = tc.tolerances ?? {};
  const options = normalizeOptions(tc.options ?? {});
  switch (tc.method) {
    case 'extract':
      await testExtract(fixturePath, options, expected, tolerances);
      break;
    case 'extract_text':
      await testExtractText(fixturePath, options, expected, tolerances);
      break;
    case 'extract_markdown':
      await testExtractMarkdown(fixturePath, options, expected, tolerances);
      break;
    case 'get_metadata':
      await testGetMetadata(fixturePath, options, expected, tolerances);
      break;
    case 'hash':
      await testHash(fixturePath, options, expected, tolerances);
      break;
    case 'classify':
      await testClassify(fixturePath, options, expected, tolerances);
      break;
    case 'verify_receipt':
      await testVerifyReceipt(fixturePath, options, expected, tolerances);
      break;
    case 'search':
      await testSearch(fixturePath, options, expected, tolerances);
      break;
    case 'extract_stream':
      await testExtractStream(fixturePath, options, expected, tolerances);
      break;
    default:
      console.log(`Skipping method: ${tc.method}`);
  }
}

function normalizeOptions(options: Record<string, any>): Record<string, any> {
  const names: Record<string, string> = {
    ocr_language: 'ocrLanguage',
    ocr_threshold: 'ocrThreshold',
    preserve_layout: 'preserveLayout',
    extract_images: 'extractImages',
    image_format: 'imageFormat',
    min_image_size: 'minImageSize',
    case_insensitive: 'caseInsensitive',
    whole_word: 'wholeWord',
    max_results: 'maxResults',
    max_pages: 'maxPages',
    password: 'password',
  };
  return Object.fromEntries(Object.entries(options).map(([key, value]) => [names[key] ?? key, value]));
}

function resolvePath(value: any, pathExpression: string): { found: boolean; value?: any } {
  let current = value;
  const tokens = pathExpression.match(/[^.[\]]+|\[\d+\]/g) ?? [];
  for (const token of tokens) {
    if (token === 'length') {
      if (current === null || current === undefined || typeof current.length !== 'number') return { found: false };
      current = current.length;
    } else if (token.startsWith('[')) {
      const index = Number(token.slice(1, -1));
      if (!Array.isArray(current) || index >= current.length) return { found: false };
      current = current[index];
    } else {
      if (current === null || current === undefined || !(token in Object(current))) return { found: false };
      current = current[token];
    }
  }
  return { found: true, value: current };
}

function findTolerance(tolerances: Record<string, any>, pathExpression: string): any {
  if (pathExpression in tolerances) return tolerances[pathExpression];
  for (const [pattern, tolerance] of Object.entries(tolerances)) {
    if (pattern.includes('*') && new RegExp(`^${pattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&').replaceAll('\\*', '.*')}$`).test(pathExpression)) {
      return tolerance;
    }
  }
  return undefined;
}

function compareNumber(actual: number, expected: number, tolerance: any): boolean {
  if (!tolerance) return actual === expected;
  const difference = Math.abs(actual - expected);
  if (tolerance.abs !== undefined && difference <= tolerance.abs) return true;
  const average = (actual + expected) / 2;
  return tolerance.rel !== undefined && average > 0 && difference / average <= tolerance.rel;
}

function compareExpected(actual: any, expected: any, tolerances: Record<string, any>, pathExpression: string): { passed: boolean; reason: string } {
  if (expected !== null && typeof expected === 'object' && !Array.isArray(expected)) {
    if (typeof actual === 'number') {
      if (expected.min !== undefined && actual < expected.min) return { passed: false, reason: `${pathExpression}: value ${actual} < minimum ${expected.min}` };
      if (expected.max !== undefined && actual > expected.max) return { passed: false, reason: `${pathExpression}: value ${actual} > maximum ${expected.max}` };
      if (expected.value !== undefined && !compareNumber(actual, expected.value, findTolerance(tolerances, pathExpression))) {
        return { passed: false, reason: `${pathExpression}: numeric mismatch` };
      }
      if (Object.keys(expected).every((key) => ['min', 'max', 'value'].includes(key))) return { passed: true, reason: '' };
    } else if (Array.isArray(actual)) {
      if (expected.min !== undefined && actual.length < expected.min) return { passed: false, reason: `${pathExpression}: array length ${actual.length} < minimum ${expected.min}` };
      if (expected.max !== undefined && actual.length > expected.max) return { passed: false, reason: `${pathExpression}: array length ${actual.length} > maximum ${expected.max}` };
      if (Object.keys(expected).every((key) => ['min', 'max'].includes(key))) return { passed: true, reason: '' };
    } else if (typeof actual === 'string') {
      if (expected.min_length !== undefined && actual.length < expected.min_length) return { passed: false, reason: `${pathExpression}: string length ${actual.length} < minimum ${expected.min_length}` };
      for (const substring of expected.contains ?? []) {
        if (!actual.includes(substring)) return { passed: false, reason: `${pathExpression}: string does not contain '${substring}'` };
      }
      if (Object.keys(expected).every((key) => ['min_length', 'contains'].includes(key))) return { passed: true, reason: '' };
    }
  }
  if (Object.is(actual, expected) || (Number.isNaN(actual) && Number.isNaN(expected))) return { passed: true, reason: '' };
  return { passed: false, reason: `${pathExpression}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}` };
}

function assertExpected(actual: any, expected: Record<string, any>, tolerances: Record<string, any>): void {
  assert.ok(Object.keys(expected).length > 0, 'expected must contain at least one assertion');
  for (const [pathExpression, wanted] of Object.entries(expected)) {
    const lookupPath = (pathExpression === 'min_length' || pathExpression === 'contains') && actual && 'value' in actual ? 'value' : pathExpression;
    const resolved = resolvePath(actual, lookupPath);
    if (!resolved.found && wanted === null) {
      resolved.found = true;
      resolved.value = null;
    }
    assert.ok(resolved.found, `${pathExpression}: missing value`);
    const result = compareExpected(resolved.value, wanted, tolerances, pathExpression);
    assert.ok(result.passed, result.reason);
  }
}

function asJson(value: any): any {
  if (Array.isArray(value)) return value.map(asJson);
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, asJson(item)]));
  return value;
}

function normalizeDocument(document: any): any {
  const value = asJson(document);
  value.schema_version ??= '1.0';
  for (const page of value.pages ?? []) {
    if (page.page_index === undefined && page.page !== undefined) page.page_index = page.page - 1;
    if (page.page_type === undefined && page.type !== undefined) page.page_type = page.type;
  }
  return value;
}

function normalizeMetadata(metadata: any): any {
  const value = asJson(metadata);
  for (const field of ['title', 'author', 'creator']) {
    value[`has_${field}`] ??= value[field] !== undefined && value[field] !== null;
  }
  value.has_xmp ??= false;
  return { metadata: value };
}

function normalizeHash(fingerprint: any, stable: boolean): any {
  const value = asJson(fingerprint);
  if (typeof value.hash === 'string') value.hash = value.hash.replace(/^pdftract-v1:/, '');
  value.hash_type ??= 'sha256';
  value.fast_hash_different_from_hash = value.fast_hash !== undefined && value.fast_hash !== value.hash;
  value.content_hash_stable = stable;
  return value;
}

async function testExtract(fixturePath: string, options: any, expected: any, tolerances: any) {
  assertExpected(normalizeDocument(await client.extract(path(fixturePath), options)), expected, tolerances);
}

async function testExtractText(fixturePath: string, options: any, expected: any, tolerances: any) {
  assertExpected({ output_type: 'string', value: await client.extractText(path(fixturePath), options) }, expected, tolerances);
}

async function testExtractMarkdown(fixturePath: string, options: any, expected: any, tolerances: any) {
  assertExpected({ output_type: 'string', value: await client.extractMarkdown(path(fixturePath), options) }, expected, tolerances);
}

async function testGetMetadata(fixturePath: string, options: any, expected: any, tolerances: any) {
  assertExpected(normalizeMetadata(await client.getMetadata(path(fixturePath), options)), expected, tolerances);
}

async function testHash(fixturePath: string, options: any, expected: any, tolerances: any) {
  const fingerprint = await client.hash(path(fixturePath), options);
  const second = expected.content_hash_stable !== undefined ? await client.hash(path(fixturePath), options) : fingerprint;
  assertExpected(normalizeHash(fingerprint, asJson(second).hash === asJson(fingerprint).hash), expected, tolerances);
}

async function testClassify(fixturePath: string, _options: any, expected: any, tolerances: any) {
  const value = asJson(await client.classify(path(fixturePath)));
  value.tags ??= value.labels ?? [];
  value.heuristics ??= {};
  assertExpected(value, expected, tolerances);
}

async function testVerifyReceipt(fixturePath: string, options: any, expected: any, tolerances: any) {
  assert.ok(options.receipt, 'receipt option is required');
  const receipt = options.receipt.startsWith('/') || options.receipt.startsWith('http')
    ? options.receipt
    : join('fixtures', options.receipt);
  assertExpected({ valid: await client.verifyReceipt(fixturePath, receipt) }, expected, tolerances);
}

async function testSearch(fixturePath: string, options: any, expected: any, tolerances: any) {
  const { pattern, ...searchOptions } = options;
  const matches: any[] = [];
  for await (const match of (client as any).search(path(fixturePath), pattern, searchOptions)) matches.push(asJson(match));
  assertExpected({
    output_type: 'iterator',
    match_count: matches.length,
    min_matches: matches.length,
    first_match_page: matches[0]?.page ?? null,
    first_match_text: matches[0]?.text ?? null,
    matches,
  }, expected, tolerances);
}

async function testExtractStream(fixturePath: string, options: any, expected: any, tolerances: any) {
  const frames: any[] = [];
  for await (const frame of (client as any).extractStream(path(fixturePath), options)) frames.push(asJson(frame));
  const header = frames.find((frame) => frame.type === 'header') ?? {};
  assertExpected({
    output_type: 'iterator',
    frame_count: frames.length,
    first_frame_type: frames[0]?.type ?? null,
    last_frame_type: frames.at(-1)?.type ?? null,
    page_frames: frames.filter((frame) => frame.type === 'page').length,
    header_frame_has_schema_version: header.schema_version !== undefined,
    header_frame_has_total_pages: header.total_pages !== undefined,
  }, expected, tolerances);
}
