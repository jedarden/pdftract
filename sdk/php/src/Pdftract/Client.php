<?php

declare(strict_types=1);

namespace Jedarden\Pdftract;

use Jedarden\Pdftract\Models\Classification;
use Jedarden\Pdftract\Models\Document;
use Jedarden\Pdftract\Models\Fingerprint;
use Jedarden\Pdftract\Models\Metadata;
use Jedarden\Pdftract\Models\Page;
use Jedarden\Pdftract\Models\Receipt;
use Psr\Log\LoggerInterface;
use Psr\Log\NullLogger;

/**
 * pdftract PHP SDK Client
 *
 * Main client for interacting with the pdftract binary.
 * Uses proc_open to spawn subprocesses and parse JSON output.
 */
class Client
{
    private string $binaryPath = 'pdftract';
    private LoggerInterface $logger;

    /**
     * Constructor
     *
     * @param LoggerInterface|null $logger PSR-3 logger for debugging (default: NullLogger)
     */
    public function __construct(?LoggerInterface $logger = null)
    {
        $this->logger = $logger ?? new NullLogger();
    }

    /**
     * Execute a pdftract command
     *
     * @param array $command CLI arguments
     * @param bool $parseJson Whether to parse output as JSON (default: true)
     * @return mixed Parsed JSON response if $parseJson is true, raw stdout otherwise
     * @throws PdftractException On command failure
     */
    private function execute(array $command, bool $parseJson = true): mixed
    {
        $cmd = escapeshellcmd($this->binaryPath);
        foreach ($command as $arg) {
            $cmd .= ' ' . escapeshellarg($arg);
        }

        $this->logger->debug('Executing pdftract command', ['command' => $cmd]);

        $descriptorspec = [
            0 => ['pipe', 'r'],
            1 => ['pipe', 'w'],
            2 => ['pipe', 'w'],
        ];

        $process = proc_open($cmd, $descriptorspec, $pipes);

        if (!is_resource($process)) {
            $error = 'Failed to start pdftract process';
            $this->logger->error('Failed to start process', ['command' => $cmd, 'error' => $error]);
            throw new PdftractException($error, -1);
        }

        fclose($pipes[0]);

        $stdout = stream_get_contents($pipes[1]);
        $stderr = stream_get_contents($pipes[2]);

        fclose($pipes[1]);
        fclose($pipes[2]);

        $exitCode = proc_close($process);

        if ($exitCode !== 0) {
            $this->logger->error('pdftract command failed', [
                'command' => $cmd,
                'exit_code' => $exitCode,
                'stderr' => $stderr
            ]);
            throw new PdftractException($stderr ?: 'Command failed with no output', $exitCode);
        }

        if ($parseJson) {
            $result = json_decode($stdout, true);
            if ($result === null && json_last_error() !== JSON_ERROR_NONE) {
                $this->logger->error('Failed to decode JSON output', [
                    'command' => $cmd,
                    'json_error' => json_last_error_msg()
                ]);
                throw new PdftractException('Failed to decode JSON output: ' . json_last_error_msg(), -1);
            }
            return $result;
        }

        return $stdout;
    }

    /**
     * Resolve source to path string
     *
     * @param string|Stringable $source Source object or path string
     * @return string Resolved path string
     */
    private function resolveSource(string|Stringable $source): string
    {
        if ($source instanceof Source) {
            return $source->toArgs()[0] ?? '';
        }
        return (string) $source;
    }

    /**
     * Convert camelCase option keys to CLI kebab-case flags
     *
     * @param array $options Options array with camelCase keys
     * @return array CLI arguments
     */
    private function convertOptions(array $options): array
    {
        $args = [];
        foreach ($options as $key => $value) {
            if ($value === null || $value === false) {
                continue;
            }

            $flag = $this->camelToKebab($key);
            $args[] = "--{$flag}";

            if ($value !== true) {
                $args[] = is_bool($value) ? ($value ? 'true' : 'false') : (string)$value;
            }
        }
        return $args;
    }

    /**
     * Convert camelCase to kebab-case
     *
     * @param string $camel camelCase string
     * @return string kebab-case string
     */
    private function camelToKebab(string $camel): string
    {
        return strtolower(preg_replace('/(?<!^)[A-Z]/', '-$0', lcfirst($camel)));
    }

    /**
     * Extract structured data from a PDF
     *
     * @param string|Stringable $source Source object or path string
     * @param array $options Options (e.g., ['ocrLanguage' => 'eng'])
     * @return Document Document object with schema_version, metadata, pages
     * @throws PdftractException On command failure
     */
    public function extract(string|Stringable $source, array $options = []): Document
    {
        $args = [$this->resolveSource($source)];
        $args = array_merge($args, $this->convertOptions($options));
        $result = $this->execute($args);

        $pages = [];
        if (isset($result['pages']) && is_array($result['pages'])) {
            foreach ($result['pages'] as $pageData) {
                $pages[] = new Page(
                    $pageData['number'] ?? 0,
                    $pageData['text'] ?? '',
                    $pageData['structure'] ?? null
                );
            }
        }

        return new Document(
            $result['path'] ?? $this->resolveSource($source),
            $result['page_count'] ?? count($pages),
            $pages
        );
    }

    /**
     * Extract plain text from a PDF
     *
     * @param string|Stringable $source Source object or path string
     * @param array $options Options (e.g., ['ocrLanguage' => 'eng'])
     * @return string Plain text content
     * @throws PdftractException On command failure
     */
    public function extractText(string|Stringable $source, array $options = []): string
    {
        $args = ['--text', $this->resolveSource($source)];
        $args = array_merge($args, $this->convertOptions($options));
        return $this->execute($args, parseJson: false);
    }

    /**
     * Extract markdown from a PDF
     *
     * @param string|Stringable $source Source object or path string
     * @param array $options Options (e.g., ['ocrLanguage' => 'eng'])
     * @return string Markdown content
     * @throws PdftractException On command failure
     */
    public function extractMarkdown(string|Stringable $source, array $options = []): string
    {
        $args = ['--md', $this->resolveSource($source)];
        $args = array_merge($args, $this->convertOptions($options));
        return $this->execute($args, parseJson: false);
    }

    /**
     * Extract structured data from a PDF as a stream
     *
     * @param string|Stringable $source Source object or path string
     * @param array $options Options (e.g., ['ocrLanguage' => 'eng'])
     * @return \Generator Yields Document objects one at a time
     * @throws PdftractException On command failure
     */
    public function extractStream(string|Stringable $source, array $options = []): \Generator
    {
        $args = [$this->resolveSource($source)];
        $args = array_merge($args, $this->convertOptions($options));

        $cmd = escapeshellcmd($this->binaryPath);
        foreach ($args as $arg) {
            $cmd .= ' ' . escapeshellarg($arg);
        }

        $this->logger->debug('Executing pdftract stream command', ['command' => $cmd]);

        $descriptorspec = [
            0 => ['pipe', 'r'],
            1 => ['pipe', 'w'],
            2 => ['pipe', 'w'],
        ];

        $process = proc_open($cmd, $descriptorspec, $pipes);

        if (!is_resource($process)) {
            $error = 'Failed to start pdftract process';
            $this->logger->error('Failed to start stream process', ['command' => $cmd, 'error' => $error]);
            throw new PdftractException($error, -1);
        }

        fclose($pipes[0]);

        while (!feof($pipes[1])) {
            $line = fgets($pipes[1]);
            if ($line === false || trim($line) === '') {
                continue;
            }

            $data = json_decode($line, true);
            if ($data !== null) {
                $pages = [];
                if (isset($data['pages']) && is_array($data['pages'])) {
                    foreach ($data['pages'] as $pageData) {
                        $pages[] = new Page(
                            $pageData['number'] ?? 0,
                            $pageData['text'] ?? '',
                            $pageData['structure'] ?? null
                        );
                    }
                }

                yield new Document(
                    $data['path'] ?? $this->resolveSource($source),
                    $data['page_count'] ?? count($pages),
                    $pages
                );
            }
        }

        $stderr = stream_get_contents($pipes[2]);
        fclose($pipes[1]);
        fclose($pipes[2]);

        $exitCode = proc_close($process);

        if ($exitCode !== 0) {
            $this->logger->error('pdftract stream command failed', [
                'command' => $cmd,
                'exit_code' => $exitCode,
                'stderr' => $stderr
            ]);
            throw new PdftractException($stderr ?: 'Stream command failed with no output', $exitCode);
        }
    }

    /**
     * Search for text patterns in a PDF
     *
     * @param string|Stringable $source Source object or path string
     * @param string $pattern Search pattern (supports regex)
     * @param array $options Options (e.g., ['caseInsensitive' => true])
     * @return \Generator Yields search matches one at a time
     * @throws PdftractException On command failure
     */
    public function search(string|Stringable $source, string $pattern, array $options = []): \Generator
    {
        $args = ['grep', $pattern, $this->resolveSource($source)];
        $args = array_merge($args, $this->convertOptions($options));

        $cmd = escapeshellcmd($this->binaryPath);
        foreach ($args as $arg) {
            $cmd .= ' ' . escapeshellarg($arg);
        }

        $this->logger->debug('Executing pdftract search command', ['command' => $cmd]);

        $descriptorspec = [
            0 => ['pipe', 'r'],
            1 => ['pipe', 'w'],
            2 => ['pipe', 'w'],
        ];

        $process = proc_open($cmd, $descriptorspec, $pipes);

        if (!is_resource($process)) {
            $error = 'Failed to start pdftract process';
            $this->logger->error('Failed to start search process', ['command' => $cmd, 'error' => $error]);
            throw new PdftractException($error, -1);
        }

        fclose($pipes[0]);

        while (!feof($pipes[1])) {
            $line = fgets($pipes[1]);
            if ($line === false || trim($line) === '') {
                continue;
            }

            $data = json_decode($line, true);
            if ($data !== null) {
                yield $data;
            }
        }

        $stderr = stream_get_contents($pipes[2]);
        fclose($pipes[1]);
        fclose($pipes[2]);

        $exitCode = proc_close($process);

        if ($exitCode !== 0) {
            $this->logger->error('pdftract search command failed', [
                'command' => $cmd,
                'exit_code' => $exitCode,
                'stderr' => $stderr
            ]);
            throw new PdftractException($stderr ?: 'Search command failed with no output', $exitCode);
        }
    }

    /**
     * Get metadata from a PDF
     *
     * @param string|Stringable $source Source object or path string
     * @param array $options Options
     * @return Metadata Metadata with page_count, dimensions, etc.
     * @throws PdftractException On command failure
     */
    public function getMetadata(string|Stringable $source, array $options = []): Metadata
    {
        $args = ['--metadata-only', $this->resolveSource($source)];
        $args = array_merge($args, $this->convertOptions($options));
        $result = $this->execute($args);
        return new Metadata(
            $result['title'] ?? '',
            $result['author'] ?? '',
            $result['subject'] ?? null,
            $result['keywords'] ?? null
        );
    }

    /**
     * Compute hash/fingerprint of a PDF
     *
     * @param string|Stringable $source Source object or path string
     * @param array $options Options (e.g., ['fast' => true])
     * @return Fingerprint Fingerprint data with hash and fast_hash
     * @throws PdftractException On command failure
     */
    public function hash(string|Stringable $source, array $options = []): Fingerprint
    {
        $args = ['hash', $this->resolveSource($source)];
        $args = array_merge($args, $this->convertOptions($options));
        $result = $this->execute($args);
        return new Fingerprint(
            $result['id'] ?? '',
            $result['page_count'] ?? 0,
            $result['content_hash'] ?? '',
            $result['structure_hash'] ?? ''
        );
    }

    /**
     * Classify a PDF document
     *
     * @param string|Stringable $source Source object or path string
     * @return Classification Classification data with document type and confidence
     * @throws PdftractException On command failure
     */
    public function classify(string|Stringable $source): Classification
    {
        $args = ['classify', $this->resolveSource($source)];
        $result = $this->execute($args);
        return new Classification(
            $result['type'] ?? 'unknown',
            $result['confidence'] ?? 0.0
        );
    }

    /**
     * Verify a processing receipt
     *
     * @param string $path Path to PDF file
     * @param Receipt $receipt Receipt object to verify
     * @return bool True if receipt is valid, false otherwise
     * @throws PdftractException On command failure
     */
    public function verifyReceipt(string $path, Receipt $receipt): bool
    {
        $args = ['verify-receipt', $path, $receipt->id];

        $cmd = escapeshellcmd($this->binaryPath);
        foreach ($args as $arg) {
            $cmd .= ' ' . escapeshellarg($arg);
        }

        $this->logger->debug('Executing pdftract verify-receipt command', ['command' => $cmd]);

        $descriptorspec = [
            0 => ['pipe', 'r'],
            1 => ['pipe', 'w'],
            2 => ['pipe', 'w'],
        ];

        $process = proc_open($cmd, $descriptorspec, $pipes);

        if (!is_resource($process)) {
            $error = 'Failed to start pdftract process';
            $this->logger->error('Failed to start verify-receipt process', ['command' => $cmd, 'error' => $error]);
            throw new PdftractException($error, -1);
        }

        fclose($pipes[0]);

        $stdout = stream_get_contents($pipes[1]);
        $stderr = stream_get_contents($pipes[2]);

        fclose($pipes[1]);
        fclose($pipes[2]);

        $exitCode = proc_close($process);

        if ($exitCode !== 0) {
            $this->logger->error('pdftract verify-receipt command failed', [
                'command' => $cmd,
                'exit_code' => $exitCode,
                'stderr' => $stderr
            ]);
            throw new PdftractException($stderr ?: 'Verify-receipt command failed with no output', $exitCode);
        }

        return trim($stdout) === 'true';
    }
}
