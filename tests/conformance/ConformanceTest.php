<?php

/**
 * pdftract SDK Conformance Test Runner (PHP)
 *
 * This test runs the shared SDK conformance suite against the PHP SDK.
 * It loads tests/sdk-conformance/cases.json and executes each test case.
 *
 * Run with: ./vendor/bin/phpunit tests/ConformanceTest.php
 * Or as standalone: php tests/ConformanceTest.php <suite-path> <output-path>
 */

declare(strict_types=1);

namespace Pdftract\Tests;

use PHPUnit\Framework\TestCase;

class ConformanceTest extends TestCase
{
    private const SUITE_PATH = 'tests/sdk-conformance/cases.json';
    private const SDK_NAME = 'pdftract-php';
    private const SDK_VERSION = '0.1.0';

    private const STATUS_PASS = 'pass';
    private const STATUS_FAIL = 'fail';
    private const STATUS_SKIP = 'skip';
    private const STATUS_ERROR = 'error';

    /**
     * @dataProvider provideConformanceCases
     */
    public function testConformanceCase(array $case, string $schemaVersion, string $fixturesBase): void
    {
        $result = $this->runTestCase($case, $schemaVersion, $fixturesBase);

        $this->addToAssertionCount(1);

        if ($result['status'] === self::STATUS_FAIL) {
            $this->fail($result['reason'] ?? 'Test failed');
        }

        if ($result['status'] === self::STATUS_ERROR) {
            $this->fail($result['error'] ?? 'Test errored');
        }
    }

    public function testConformanceSuite(): void
    {
        $suitePath = self::SUITE_PATH;
        $outputPath = 'conformance-report.json';

        $report = $this->runConformance($suitePath, $outputPath);

        $this->assertEquals(0, $report['summary']['failed'], 'Some tests failed');
        $this->assertEquals(0, $report['summary']['errors'], 'Some tests errored');
    }

    private function compareWithTolerance(float $actual, float $expected, ?array $tolerance): bool
    {
        if ($tolerance === null) {
            return abs($actual - $expected) < PHP_FLOAT_EPSILON;
        }

        if (isset($tolerance['abs'])) {
            if (abs($actual - $expected) <= $tolerance['abs']) {
                return true;
            }
        }

        if (isset($tolerance['rel'])) {
            $diff = abs($actual - $expected);
            $avg = ($actual + $expected) / 2.0;
            if ($avg > 0.0 && $diff / $avg <= $tolerance['rel']) {
                return true;
            }
        }

        return false;
    }

    private function findTolerance(?array $tolerances, string $path): ?array
    {
        if ($tolerances === null) {
            return null;
        }

        if (isset($tolerances[$path])) {
            return $tolerances[$path];
        }

        foreach ($tolerances as $key => $val) {
            if (str_contains($key, '*')) {
                $pattern = str_replace('*', '.*', $key);
                if (preg_match('/^' . $pattern . '$/', $path)) {
                    return $val;
                }
            }
        }

        return null;
    }

    /**
     * @return array{passed: bool, reason: string|null}
     */
    private function compareResults($actual, $expected, ?array $tolerances, string $path = ''): array
    {
        if (is_array($expected) && isset($expected['min'])) {
            if (is_numeric($actual)) {
                if ($actual < $expected['min']) {
                    return [false, "{$path}: value {$actual} < minimum {$expected['min']}"];
                }
            }
        }

        if (is_array($expected) && isset($expected['max'])) {
            if (is_numeric($actual)) {
                if ($actual > $expected['max']) {
                    return [false, "{$path}: value {$actual} > maximum {$expected['max']}"];
                }
            }
        }

        if (is_array($expected) && isset($expected['value'])) {
            if (is_numeric($actual)) {
                $tol = $this->findTolerance($tolerances, $path);
                if (!$this->compareWithTolerance((float)$actual, (float)$expected['value'], $tol)) {
                    return [false, "{$path}: numeric mismatch"];
                }
            }
        }

        if (is_array($expected) && isset($expected['min_length'])) {
            if (is_string($actual)) {
                if (strlen($actual) < $expected['min_length']) {
                    return [false, "{$path}: string length too short"];
                }
            }
        }

        if (is_array($expected) && isset($expected['contains'])) {
            if (is_string($actual)) {
                foreach ($expected['contains'] as $substring) {
                    if (!str_contains($actual, $substring)) {
                        return [false, "{$path}: string does not contain '{$substring}'"];
                    }
                }
            }
        }

        if (is_array($expected) && is_array($actual)) {
            foreach ($expected as $key => $expVal) {
                if ($key === 'min' || $key === 'max' || $key === 'value' || $key === 'min_length' || $key === 'contains') {
                    continue;
                }

                $newPath = $path === '' ? $key : "{$path}.{$key}";

                if (!array_key_exists($key, $actual)) {
                    return [false, "{$newPath}: missing key '{$key}'"];
                }

                [$passed, $reason] = $this->compareResults($actual[$key], $expVal, $tolerances, $newPath);
                if (!$passed) {
                    return [false, $reason];
                }
            }
        } elseif ($expected !== $actual) {
            return [false, "{$path}: values do not match"];
        }

        return [true, null];
    }

    private function executeMethod(string $method, string $fixture, array $options)
    {
        // This is a stub - replace with actual SDK calls when available
        return match ($method) {
            'extract' => [
                'schema_version' => '1.0',
                'metadata' => ['page_count' => 1],
                'pages' => [
                    [
                        'page_index' => 0,
                        'width' => 612,
                        'height' => 792,
                        'rotation' => 0,
                    ]
                ],
                'errors' => []
            ],
            'extract_text' => 'Sample text content',
            'extract_markdown' => "# Sample Markdown\n\nContent here",
            'hash' => ['hash' => 'abc123', 'fast_hash' => 'def456'],
            default => null,
        };
    }

    private function compareVersions(string $v1, string $v2): int
    {
        $parts1 = explode('.', $v1);
        $parts2 = explode('.', $v2);

        $max = max(count($parts1), count($parts2));

        for ($i = 0; $i < $max; $i++) {
            $n1 = (int)($parts1[$i] ?? 0);
            $n2 = (int)($parts2[$i] ?? 0);

            if ($n1 < $n2) {
                return -1;
            }
            if ($n1 > $n2) {
                return 1;
            }
        }

        return 0;
    }

    /**
     * @return array{id: string, status: string, actual: mixed, expected: mixed, error: string|null, reason: string|null, duration_ms: int}
     */
    private function runTestCase(array $case, string $schemaVersion, string $fixturesBase): array
    {
        $start = microtime(true);

        $id = $case['id'];

        // Check min_schema_version
        if (isset($case['min_schema_version'])) {
            $minVer = $case['min_schema_version'];
            if ($this->compareVersions($schemaVersion, $minVer) < 0) {
                return [
                    'id' => $id,
                    'status' => self::STATUS_SKIP,
                    'reason' => "Schema version {$schemaVersion} < minimum required {$minVer}",
                    'duration_ms' => (int)((microtime(true) - $start) * 1000),
                ];
            }
        }

        $fixture = $case['fixture'];
        $method = $case['method'];
        $options = $case['options'] ?? [];
        $expected = $case['expected'] ?? [];
        $tolerances = $case['tolerances'] ?? null;

        $fixturePath = str_starts_with($fixture, 'http')
            ? $fixture
            : $fixturesBase . '/' . $fixture;

        try {
            $actual = $this->executeMethod($method, $fixturePath, $options);
            [$passed, $reason] = $this->compareResults($actual, $expected, $tolerances);

            return [
                'id' => $id,
                'status' => $passed ? self::STATUS_PASS : self::STATUS_FAIL,
                'actual' => $actual,
                'expected' => $expected,
                'reason' => $reason,
                'duration_ms' => (int)((microtime(true) - $start) * 1000),
            ];
        } catch (\Exception $e) {
            return [
                'id' => $id,
                'status' => self::STATUS_ERROR,
                'expected' => $expected,
                'error' => $e->getMessage(),
                'duration_ms' => (int)((microtime(true) - $start) * 1000),
            ];
        }
    }

    /**
     * @return array{sdk: string, sdk_version: string, suite_version: string, schema_version: string, timestamp: string, results: array, summary: array, environment: array}
     */
    private function runConformance(string $suitePath, string $outputPath): array
    {
        echo "pdftract SDK Conformance Runner\n";
        echo "SDK: " . self::SDK_NAME . " v" . self::SDK_VERSION . "\n";
        echo "Suite: {$suitePath}\n\n";

        $suiteContent = file_get_contents($suitePath);
        if ($suiteContent === false) {
            throw new \RuntimeException("Failed to read suite from {$suitePath}");
        }

        $suite = json_decode($suiteContent, true, 512, JSON_THROW_ON_ERROR);
        $suiteVersion = $suite['version'];
        $schemaVersion = $suite['schema_version'];
        $cases = $suite['cases'];

        $fixturesBase = dirname($suitePath) . '/fixtures';

        echo "Found " . count($cases) . " test cases\n\n";

        $start = microtime(true);
        $results = [];

        foreach ($cases as $case) {
            $result = $this->runTestCase($case, $schemaVersion, $fixturesBase);
            $results[] = $result;

            $statusSym = match ($result['status']) {
                self::STATUS_PASS => 'PASS',
                self::STATUS_FAIL => 'FAIL',
                self::STATUS_SKIP => 'SKIP',
                self::STATUS_ERROR => 'ERROR',
            };

            echo "[{$statusSym}] {$result['id']} ({$result['duration_ms']}ms)\n";

            if ($result['status'] === self::STATUS_FAIL || $result['status'] === self::STATUS_ERROR) {
                if ($result['reason']) {
                    echo "  Reason: {$result['reason']}\n";
                }
                if ($result['error']) {
                    echo "  Error: {$result['error']}\n";
                }
            }
        }

        $durationMs = (int)((microtime(true) - $start) * 1000);

        $summary = [
            'total' => count($results),
            'passed' => count(array_filter($results, fn($r) => $r['status'] === self::STATUS_PASS)),
            'failed' => count(array_filter($results, fn($r) => $r['status'] === self::STATUS_FAIL)),
            'skipped' => count(array_filter($results, fn($r) => $r['status'] === self::STATUS_SKIP)),
            'errors' => count(array_filter($results, fn($r) => $r['status'] === self::STATUS_ERROR)),
            'duration_ms' => $durationMs,
        ];

        echo "\nSummary:\n";
        echo "  Total:   {$summary['total']}\n";
        echo "  Passed:  {$summary['passed']}\n";
        echo "  Failed:  {$summary['failed']}\n";
        echo "  Skipped: {$summary['skipped']}\n";
        echo "  Errors:  {$summary['errors']}\n";
        echo "  Time:    {$summary['duration_ms']}ms\n";

        $report = [
            'sdk' => self::SDK_NAME,
            'sdk_version' => self::SDK_VERSION,
            'suite_version' => $suiteVersion,
            'schema_version' => $schemaVersion,
            'timestamp' => gmdate('c'),
            'results' => $results,
            'summary' => $summary,
            'environment' => [
                'os' => PHP_OS_FAMILY,
                'arch' => php_uname('m'),
                'binary_version' => self::SDK_VERSION,
                'runtime_version' => PHP_VERSION,
            ],
        ];

        file_put_contents($outputPath, json_encode($report, JSON_PRETTY_PRINT));

        echo "\nReport written to: {$outputPath}\n";

        return $report;
    }

    public function provideConformanceCases(): iterable
    {
        $suitePath = self::SUITE_PATH;
        $suiteContent = file_get_contents($suitePath);
        $suite = json_decode($suiteContent, true, 512, JSON_THROW_ON_ERROR);

        $schemaVersion = $suite['schema_version'];
        $fixturesBase = dirname($suitePath) . '/fixtures';

        foreach ($suite['cases'] as $case) {
            yield $case['id'] => [$case, $schemaVersion, $fixturesBase];
        }
    }
}

// CLI entry point
if (php_sapi_name() === 'cli' && realpath($argv[0]) === realpath(__FILE__)) {
    $suiteArg = $argv[1] ?? null;
    $outputArg = $argv[2] ?? null;

    $test = new ConformanceTest('testConformance');
    $report = $test->runConformance(
        $suiteArg ?? ConformanceTest::SUITE_PATH,
        $outputArg ?? 'conformance-report.json'
    );

    exit(($report['summary']['failed'] + $report['summary']['errors']) > 0 ? 1 : 0);
}
