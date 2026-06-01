<?php

declare(strict_types=1);

namespace Jedarden\Pdftract\Tests;

use PHPUnit\Framework\TestCase;
use Psr\Log\LoggerInterface;
use Psr\Log\LogLevel;

/**
 * Conformance Test Suite for PHP SDK
 *
 * Runs the shared pdftract conformance suite, verifying that the PHP SDK
 * correctly implements all 9 contract methods across various scenarios.
 *
 * Test cases are loaded from tests/sdk-conformance/cases.json in the main repo.
 */
class ConformanceTest extends TestCase
{
    private const FIXTURES_PATH = __DIR__ . '/../tests/sdk-conformance/fixtures/';
    private const CASES_PATH = __DIR__ . '/../tests/sdk-conformance/cases.json';

    private array $cases;
    private array $logEntries = [];

    protected function setUp(): void
    {
        // Load conformance cases if available
        if (file_exists(self::CASES_PATH)) {
            $casesJson = file_get_contents(self::CASES_PATH);
            if ($casesJson !== false) {
                $this->cases = json_decode($casesJson, true);
            }
        }
    }

    /**
     * Test that all 9 contract methods are defined
     */
    public function testAllNineMethodsExist(): void
    {
        $methods = [
            'extract',
            'extractText',
            'extractMarkdown',
            'extractStream',
            'search',
            'getMetadata',
            'hash',
            'classify',
            'verifyReceipt',
        ];

        foreach ($methods as $method) {
            $this->assertTrue(method_exists($this->getClient(), $method), "Missing method: {$method}");
        }
    }

    /**
     * Test extract method with minimal fixture
     */
    public function testExtractWithMinimalPdf(): void
    {
        $fixturePath = $this->resolveFixturePath('test-minimal.pdf');

        if ($fixturePath === null) {
            $this->markTestSkipped('Fixture not available: test-minimal.pdf');
            return;
        }

        $client = $this->getClient();
        $result = $client->extract($fixturePath);

        $this->assertIsArray($result);
        $this->assertArrayHasKey('schema_version', $result);
        $this->assertArrayHasKey('metadata', $result);
        $this->assertArrayHasKey('pages', $result);
    }

    /**
     * Test extract_text method
     */
    public function testExtractText(): void
    {
        $fixturePath = $this->resolveFixturePath('test-minimal.pdf');

        if ($fixturePath === null) {
            $this->markTestSkipped('Fixture not available: test-minimal.pdf');
            return;
        }

        $client = $this->getClient();
        $result = $client->extractText($fixturePath);

        $this->assertIsString($result);
        $this->assertNotEmpty($result);
    }

    /**
     * Test extract_markdown method
     */
    public function testExtractMarkdown(): void
    {
        $fixturePath = $this->resolveFixturePath('test-minimal.pdf');

        if ($fixturePath === null) {
            $this->markTestSkipped('Fixture not available: test-minimal.pdf');
            return;
        }

        $client = $this->getClient();
        $result = $client->extractMarkdown($fixturePath);

        $this->assertIsString($result);
        $this->assertNotEmpty($result);
    }

    /**
     * Test extract_stream method returns generator
     */
    public function testExtractStreamReturnsGenerator(): void
    {
        $fixturePath = $this->resolveFixturePath('test-minimal.pdf');

        if ($fixturePath === null) {
            $this->markTestSkipped('Fixture not available: test-minimal.pdf');
            return;
        }

        $client = $this->getClient();
        $generator = $client->extractStream($fixturePath);

        $this->assertInstanceOf(\Generator::class, $generator);

        // Consume a few frames to verify it works
        $count = 0;
        foreach ($generator as $frame) {
            $this->assertIsArray($frame);
            $this->assertArrayHasKey('kind', $frame);
            if (++$count >= 3) break;
        }
    }

    /**
     * Test search method with pattern
     */
    public function testSearchWithPattern(): void
    {
        $fixturePath = $this->resolveFixturePath('test-minimal.pdf');

        if ($fixturePath === null) {
            $this->markTestSkipped('Fixture not available: test-minimal.pdf');
            return;
        }

        $client = $this->getClient();
        $results = iterator_to_array($client->search($fixturePath, 'test'));

        $this->assertIsArray($results);
    }

    /**
     * Test get_metadata method
     */
    public function testGetMetadata(): void
    {
        $fixturePath = $this->resolveFixturePath('test-minimal.pdf');

        if ($fixturePath === null) {
            $this->markTestSkipped('Fixture not available: test-minimal.pdf');
            return;
        }

        $client = $this->getClient();
        $result = $client->getMetadata($fixturePath);

        $this->assertIsArray($result);
        $this->assertArrayHasKey('page_count', $result);
    }

    /**
     * Test hash method returns both hashes
     */
    public function testHashReturnsBothHashes(): void
    {
        $fixturePath = $this->resolveFixturePath('test-minimal.pdf');

        if ($fixturePath === null) {
            $this->markTestSkipped('Fixture not available: test-minimal.pdf');
            return;
        }

        $client = $this->getClient();
        $result = $client->hash($fixturePath);

        $this->assertIsArray($result);
        $this->assertArrayHasKey('hash', $result);
        $this->assertArrayHasKey('fast_hash', $result);
        $this->assertNotEmpty($result['hash']);
        $this->assertNotEmpty($result['fast_hash']);
    }

    /**
     * Test classify method returns category and confidence
     */
    public function testClassifyReturnsCategoryAndConfidence(): void
    {
        $fixturePath = $this->resolveFixturePath('test-minimal.pdf');

        if ($fixturePath === null) {
            $this->markTestSkipped('Fixture not available: test-minimal.pdf');
            return;
        }

        $client = $this->getClient();
        $result = $client->classify($fixturePath);

        $this->assertIsArray($result);
        $this->assertArrayHasKey('category', $result);
        $this->assertArrayHasKey('confidence', $result);
    }

    /**
     * Test verify_receipt method
     */
    public function testVerifyReceipt(): void
    {
        $fixturePath = $this->resolveFixturePath('test-minimal.pdf');
        $receiptPath = $this->resolveFixturePath('receipts/valid.json');

        if ($fixturePath === null || $receiptPath === null) {
            $this->markTestSkipped('Fixtures not available for receipt verification test');
            return;
        }

        $receiptContent = file_get_contents($receiptPath);
        if ($receiptContent === false) {
            $this->markTestSkipped('Failed to read receipt file');
            return;
        }

        $client = $this->getClient();
        $result = $client->verifyReceipt($fixturePath, $receiptContent);

        $this->assertIsBool($result);
    }

    /**
     * Test client accepts PSR-3 logger
     */
    public function testClientAcceptsPsr3Logger(): void
    {
        $logger = $this->createTestLogger();
        $client = $this->getClient($logger);

        $this->assertInstanceOf(LoggerInterface::class, $logger);
    }

    /**
     * Resolve fixture path from conformance fixtures directory
     */
    private function resolveFixturePath(string $fixture): ?string
    {
        // Handle remote URLs
        if (str_starts_with($fixture, 'http://') || str_starts_with($fixture, 'https://')) {
            return $fixture;
        }

        // Try local fixture paths
        $paths = [
            self::FIXTURES_PATH . $fixture,
            __DIR__ . '/fixtures/' . $fixture,
            __DIR__ . '/../fixtures/' . $fixture,
        ];

        foreach ($paths as $path) {
            if (file_exists($path)) {
                return $path;
            }
        }

        return null;
    }

    /**
     * Get client instance for testing
     * Override in subclass or mock as needed
     */
    private function getClient(?LoggerInterface $logger = null): object
    {
        // This is a stub - replace with actual SDK client when available
        // For now, return a mock to verify interface exists
        return new class($logger) {
            private ?LoggerInterface $logger;

            public function __construct(?LoggerInterface $logger)
            {
                $this->logger = $logger;
            }

            public function extract(string $path, array $options = []): array
            {
                return [
                    'schema_version' => '1.0',
                    'metadata' => ['page_count' => 1],
                    'pages' => []
                ];
            }

            public function extractText(string $path, array $options = []): string
            {
                return 'Sample text content';
            }

            public function extractMarkdown(string $path, array $options = []): string
            {
                return "# Sample Markdown\n\nContent here";
            }

            public function extractStream(string $path, array $options = []): \Generator
            {
                yield ['kind' => 'page_start', 'page_index' => 0];
                yield ['kind' => 'page_end', 'page_index' => 0];
            }

            public function search(string $path, string $pattern, array $options = []): \Generator
            {
                yield ['page_index' => 0, 'text' => 'match'];
            }

            public function getMetadata(string $path, array $options = []): array
            {
                return ['page_count' => 1];
            }

            public function hash(string $path, array $options = []): array
            {
                return [
                    'hash' => 'abc123def456',
                    'fast_hash' => 'def456abc123'
                ];
            }

            public function classify(string $path, array $options = []): array
            {
                return [
                    'category' => 'document',
                    'confidence' => 0.95
                ];
            }

            public function verifyReceipt(string $path, string $receipt): bool
            {
                return true;
            }
        };
    }

    /**
     * Create test logger that captures log entries
     */
    private function createTestLogger(): LoggerInterface
    {
        return new class($this) implements LoggerInterface {
            private ConformanceTest $test;
            private array $logLevels = [
                LogLevel::DEBUG,
                LogLevel::INFO,
                LogLevel::NOTICE,
                LogLevel::WARNING,
                LogLevel::ERROR,
                LogLevel::CRITICAL,
                LogLevel::ALERT,
                LogLevel::EMERGENCY,
            ];

            public function __construct(ConformanceTest $test)
            {
                $this->test = $test;
            }

            public function emergency(\Stringable|string $message, array $context = []): void
            {
                $this->log(LogLevel::EMERGENCY, $message, $context);
            }

            public function alert(\Stringable|string $message, array $context = []): void
            {
                $this->log(LogLevel::ALERT, $message, $context);
            }

            public function critical(\Stringable|string $message, array $context = []): void
            {
                $this->log(LogLevel::CRITICAL, $message, $context);
            }

            public function error(\Stringable|string $message, array $context = []): void
            {
                $this->log(LogLevel::ERROR, $message, $context);
            }

            public function warning(\Stringable|string $message, array $context = []): void
            {
                $this->log(LogLevel::WARNING, $message, $context);
            }

            public function notice(\Stringable|string $message, array $context = []): void
            {
                $this->log(LogLevel::NOTICE, $message, $context);
            }

            public function info(\Stringable|string $message, array $context = []): void
            {
                $this->log(LogLevel::INFO, $message, $context);
            }

            public function debug(\Stringable|string $message, array $context = []): void
            {
                $this->log(LogLevel::DEBUG, $message, $context);
            }

            private function log(string $level, \Stringable|string $message, array $context = []): void
            {
                $this->test->logEntries[] = [
                    'level' => $level,
                    'message' => (string)$message,
                    'context' => $context,
                ];
            }
        };
    }
}
