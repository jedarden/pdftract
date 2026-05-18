/**
 * pdftract SDK Conformance Test Runner (Node.js / TypeScript)
 *
 * This test runs the shared SDK conformance suite against the Node.js SDK.
 * It loads tests/sdk-conformance/cases.json and executes each test case.
 *
 * Run with: vitest test/conformance/conformance.test.ts
 * Or as standalone: ts-node test/conformance/conformance.test.ts
 */

import { readFileSync, writeFileSync } from 'fs';
import { join } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = join(__filename, '..');

const SUITE_PATH = join(__dirname, '..', '..', 'sdk-conformance', 'cases.json');
const SDK_NAME = 'pdftract-node';
const SDK_VERSION = '0.1.0';

enum TestStatus {
  Pass = 'pass',
  Fail = 'fail',
  Skip = 'skip',
  Error = 'error',
}

interface TestResult {
  id: string;
  status: TestStatus;
  actual?: any;
  expected?: any;
  error?: string;
  reason?: string;
  duration_ms: number;
}

interface ConformanceReport {
  sdk: string;
  sdk_version: string;
  suite_version: string;
  schema_version: string;
  timestamp: string;
  results: TestResult[];
  summary: {
    total: number;
    passed: number;
    failed: number;
    skipped: number;
    errors: number;
    duration_ms: number;
  };
  environment: {
    os: string;
    arch: string;
    binary_version: string;
    runtime_version: string;
  };
}

interface SuiteCase {
  id: string;
  fixture: string;
  method: string;
  options: Record<string, any>;
  expected: any;
  tolerances?: Record<string, { abs?: number; rel?: number }>;
  feature?: string;
  min_schema_version?: string;
  skip_reason?: string;
}

interface Suite {
  version: string;
  schema_version: string;
  cases: SuiteCase[];
}

function loadSuite(path: string): Suite {
  const content = readFileSync(path, 'utf-8');
  return JSON.parse(content);
}

function compareWithTolerance(
  actual: number,
  expected: number,
  tolerance?: { abs?: number; rel?: number }
): boolean {
  if (!tolerance) {
    return Math.abs(actual - expected) < Number.EPSILON;
  }

  if (tolerance.abs !== undefined) {
    if (Math.abs(actual - expected) <= tolerance.abs) {
      return true;
    }
  }

  if (tolerance.rel !== undefined) {
    const diff = Math.abs(actual - expected);
    const avg = (actual + expected) / 2.0;
    if (avg > 0.0 && diff / avg <= tolerance.rel) {
      return true;
    }
  }

  return false;
}

function findTolerance(
  tolerances: Record<string, any> | undefined,
  path: string
): { abs?: number; rel?: number } | undefined {
  if (!tolerances) {
    return undefined;
  }

  if (path in tolerances) {
    return tolerances[path];
  }

  for (const [key, val] of Object.entries(tolerances)) {
    if (key.includes('*')) {
      const pattern = key.replace(/\*/g, '.*');
      const regex = new RegExp(pattern);
      if (regex.test(path)) {
        return val;
      }
    }
  }

  return undefined;
}

function compareResults(
  actual: any,
  expected: any,
  tolerances: Record<string, any> | undefined,
  path: string = ''
): { passed: boolean; reason?: string } {
  if (typeof expected === 'object' && expected !== null && !Array.isArray(expected)) {
    if ('min' in expected && typeof actual === 'number') {
      if (actual < expected.min) {
        return { passed: false, reason: `${path}: value ${actual} < minimum ${expected.min}` };
      }
    }
    if ('max' in expected && typeof actual === 'number') {
      if (actual > expected.max) {
        return { passed: false, reason: `${path}: value ${actual} > maximum ${expected.max}` };
      }
    }
    if ('value' in expected && typeof actual === 'number') {
      const tol = findTolerance(tolerances, path);
      if (!compareWithTolerance(actual, expected.value, tol)) {
        return { passed: false, reason: `${path}: numeric mismatch` };
      }
    }
    if ('min_length' in expected && typeof actual === 'string') {
      if (actual.length < expected.min_length) {
        return { passed: false, reason: `${path}: string length ${actual.length} < minimum ${expected.min_length}` };
      }
    }
    if ('contains' in expected && typeof actual === 'string') {
      for (const substring of expected.contains) {
        if (!actual.includes(substring)) {
          return { passed: false, reason: `${path}: string does not contain '${substring}'` };
        }
      }
    }
    if ('min' in expected && Array.isArray(actual)) {
      if (actual.length < expected.min) {
        return { passed: false, reason: `${path}: array length ${actual.length} < minimum ${expected.min}` };
      }
    }
    if ('max' in expected && Array.isArray(actual)) {
      if (actual.length > expected.max) {
        return { passed: false, reason: `${path}: array length ${actual.length} > maximum ${expected.max}` };
      }
    }

    // Nested object comparison
    if (typeof actual === 'object' && actual !== null) {
      for (const [key, expVal] of Object.entries(expected)) {
        const newPath = path ? `${path}.${key}` : key;
        if (!(key in actual)) {
          return { passed: false, reason: `${newPath}: missing key '${key}'` };
        }
        const result = compareResults(actual[key], expVal, tolerances, newPath);
        if (!result.passed) {
          return result;
        }
      }
    }
  } else if (Array.isArray(expected) && Array.isArray(actual)) {
    for (let i = 0; i < expected.length; i++) {
      const newPath = `${path}[${i}]`;
      if (i >= actual.length) {
        return { passed: false, reason: `${newPath}: missing index` };
      }
      const result = compareResults(actual[i], expected[i], tolerances, newPath);
      if (!result.passed) {
        return result;
      }
    }
  } else {
    if (actual !== expected) {
      return { passed: false, reason: `${path}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}` };
    }
  }

  return { passed: true };
}

async function executeMethod(
  method: string,
  fixture: string,
  options: Record<string, any>
): Promise<any> {
  // This is a stub - replace with actual SDK calls when available
  switch (method) {
    case 'extract':
      return {
        schema_version: '1.0',
        metadata: { page_count: 1 },
        pages: [
          {
            page_index: 0,
            width: 612,
            height: 792,
            rotation: 0,
          },
        ],
        errors: [],
      };
    case 'extract_text':
      return 'Sample text content';
    case 'extract_markdown':
      return '# Sample Markdown\n\nContent here';
    case 'extract_stream':
      return { output_type: 'iterator', frame_count: 3 };
    case 'search':
      return { output_type: 'iterator', matches: [{ page: 0, text: 'found' }] };
    case 'get_metadata':
      return { metadata: { page_count: 1, title: 'Test', author: 'Test' } };
    case 'hash':
      return { hash: 'abc123', fast_hash: 'def456' };
    case 'classify':
      return { category: 'scientific_paper', confidence: 0.85, tags: ['academic'] };
    case 'verify_receipt':
      return { valid: true };
    default:
      return null;
  }
}

async function runTestCase(
  case: SuiteCase,
  schemaVersion: string,
  fixturesBase: string
): Promise<TestResult> {
  const startTime = Date.now();

  // Check min_schema_version
  if (case.min_schema_version) {
    const [major, minor] = schemaVersion.split('.').map(Number);
    const [minMajor, minMinor] = case.min_schema_version.split('.').map(Number);
    if (major < minMajor || (major === minMajor && minor < minMinor)) {
      return {
        id: case.id,
        status: TestStatus.Skip,
        reason: `Schema version ${schemaVersion} < minimum required ${case.min_schema_version}`,
        duration_ms: Date.now() - startTime,
      };
    }
  }

  const fixturePath = case.fixture.startsWith('http')
    ? case.fixture
    : join(fixturesBase, case.fixture);

  try {
    const actual = await executeMethod(case.method, fixturePath, case.options);
    const { passed, reason } = compareResults(actual, case.expected, case.tolerances);

    return {
      id: case.id,
      status: passed ? TestStatus.Pass : TestStatus.Fail,
      actual,
      expected: case.expected,
      reason,
      duration_ms: Date.now() - startTime,
    };
  } catch (e) {
    return {
      id: case.id,
      status: TestStatus.Error,
      expected: case.expected,
      error: e instanceof Error ? e.message : String(e),
      duration_ms: Date.now() - startTime,
    };
  }
}

export async function runConformance(
  suitePath: string = SUITE_PATH,
  outputPath: string = 'conformance-report.json'
): Promise<ConformanceReport> {
  const os = process.platform;
  const arch = process.arch;
  const runtimeVersion = `Node.js ${process.version}`;

  console.log(`pdftract SDK Conformance Runner`);
  console.log(`SDK: ${SDK_NAME} v${SDK_VERSION}`);
  console.log(`Suite: ${suitePath}`);
  console.log();

  const suite = loadSuite(suitePath);
  const fixturesBase = join(suitePath, '..', 'fixtures');

  console.log(`Found ${suite.cases.length} test cases`);
  console.log();

  const startTime = Date.now();
  const results: TestResult[] = [];

  for (const case_ of suite.cases) {
    const result = await runTestCase(case_, suite.schema_version, fixturesBase);
    const statusSym = {
      [TestStatus.Pass]: 'PASS',
      [TestStatus.Fail]: 'FAIL',
      [TestStatus.Skip]: 'SKIP',
      [TestStatus.Error]: 'ERROR',
    }[result.status];

    console.log(`[${statusSym}] ${result.id} (${result.duration_ms}ms)`);

    if (result.status === TestStatus.Fail || result.status === TestStatus.Error) {
      if (result.reason) {
        console.log(`  Reason: ${result.reason}`);
      }
      if (result.error) {
        console.log(`  Error: ${result.error}`);
      }
    }

    results.push(result);
  }

  const duration_ms = Date.now() - startTime;

  const summary = {
    total: results.length,
    passed: results.filter((r) => r.status === TestStatus.Pass).length,
    failed: results.filter((r) => r.status === TestStatus.Fail).length,
    skipped: results.filter((r) => r.status === TestStatus.Skip).length,
    errors: results.filter((r) => r.status === TestStatus.Error).length,
    duration_ms,
  };

  console.log();
  console.log('Summary:');
  console.log(`  Total:   ${summary.total}`);
  console.log(`  Passed:  ${summary.passed}`);
  console.log(`  Failed:  ${summary.failed}`);
  console.log(`  Skipped: ${summary.skipped}`);
  console.log(`  Errors:  ${summary.errors}`);
  console.log(`  Time:    ${summary.duration_ms}ms`);

  const report: ConformanceReport = {
    sdk: SDK_NAME,
    sdk_version: SDK_VERSION,
    suite_version: suite.version,
    schema_version: suite.schema_version,
    timestamp: new Date().toISOString(),
    results,
    summary,
    environment: {
      os,
      arch,
      binary_version: SDK_VERSION,
      runtime_version: runtimeVersion,
    },
  };

  writeFileSync(outputPath, JSON.stringify(report, null, 2));
  console.log();
  console.log(`Report written to: ${outputPath}`);

  return report;
}

// Vitest entry point
export async function testConformanceSuite() {
  const report = await runConformance();
  if (report.summary.failed > 0) {
    throw new Error(`${report.summary.failed} tests failed`);
  }
  if (report.summary.errors > 0) {
    throw new Error(`${report.summary.errors} tests errored`);
  }
}

// CLI entry point
if (import.meta.url === `file://${process.argv[1]}`) {
  const suiteArg = process.argv[2];
  const outputArg = process.argv[3];

  runConformance(suiteArg, outputArg).then((report) => {
    process.exit(report.summary.failed === 0 && report.summary.errors === 0 ? 0 : 1);
  });
}
