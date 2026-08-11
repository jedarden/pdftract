<?php

namespace Jedarden\Pdftract;

use Jedarden\Pdftract\Models\Document;
use Jedarden\Pdftract\Models\Page;
use Jedarden\Pdftract\Models\Match;
use Jedarden\Pdftract\Models\Metadata;
use Jedarden\Pdftract\Models\Fingerprint;
use Jedarden\Pdftract\Models\Classification;
use Psr\Log\LoggerInterface;
use Psr\Log\NullLogger;

/**
 * Main client class for pdftract SDK.
 */
class Client
{
    private string $binaryPath;
    private string $version;
    private LoggerInterface $logger;

    /**
     * Create a new Client instance.
     *
     * @param string $binaryPath Path to pdftract binary (default: searches PATH)
     * @param LoggerInterface|null $logger Optional PSR-3 logger
     * @throws PdftractException if binary not found
     */
    public function __construct(string $binaryPath = 'pdftract', ?LoggerInterface $logger = null)
    {
        $this->binaryPath = $this->findBinary($binaryPath);
        $this->version = '1.0.0';
        $this->logger = $logger ?? new NullLogger();
    }

    /**
     * Find the pdftract binary in PATH or use the provided path.
     */
    private function findBinary(string $binaryPath): string
    {
        // If it's an absolute path, use it directly
        if (str_starts_with($binaryPath, '/') || str_starts_with($binaryPath, '.')) {
            if (!file_exists($binaryPath)) {
                throw new PdftractException("pdftract binary not found at: $binaryPath", 1);
            }
            return $binaryPath;
        }

        // Search in PATH
        $paths = explode(':', getenv('PATH') ?: '');
        foreach ($paths as $path) {
            $fullPath = rtrim($path, '/') . '/' . $binaryPath;
            if (file_exists($fullPath) && is_executable($fullPath)) {
                return $fullPath;
            }
        }

        throw new PdftractException("pdftract binary not found in PATH: $binaryPath", 1);
    }

    /**
     * Execute a pdftract command and return the output.
     *
     * @param array $args Command arguments
     * @param bool $json Whether to parse output as JSON
     * @return string|array|null Output or parsed JSON
     * @throws PdftractException on failure
     */
    private function exec(array $args, bool $json = true)
    {
        $cmd = escapeshellcmd($this->binaryPath);
        foreach ($args as $arg) {
            $cmd .= ' ' . escapeshellarg($arg);
        }

        $this->logger->debug("Executing: {cmd}", ['cmd' => $cmd]);

        $descriptorspec = [
            0 => ['pipe', 'r'],
            1 => ['pipe', 'w'],
            2 => ['pipe', 'w'],
        ];

        $process = proc_open($cmd, $descriptorspec, $pipes);

        if (!is_resource($process)) {
            $error = "Failed to start process: $cmd";
            $this->logger->error($error);
            throw new PdftractException($error, -1);
        }

        fclose($pipes[0]);

        $stdout = stream_get_contents($pipes[1]);
        $stderr = stream_get_contents($pipes[2]);

        fclose($pipes[1]);
        fclose($pipes[2]);

        $exitCode = proc_close($process);

        if ($exitCode !== 0) {
            $this->logger->error("Command failed with exit code {code}: {stderr}", [
                'code' => $exitCode,
                'stderr' => $stderr
            ]);
            throw $this->mapError($stderr ?: $stdout, $exitCode);
        }

        if ($json) {
            $result = json_decode($stdout, true);
            if (json_last_error() !== JSON_ERROR_NONE) {
                $error = "Failed to decode JSON output: " . json_last_error_msg();
                $this->logger->error($error, ['output' => $stdout]);
                throw new PdftractException($error, -1);
            }
            return $result;
        }

        return $stdout;
    }

    /**
     * Map exit code to appropriate exception.
     */
    private function mapError(string $stderr, int $exitCode): PdftractException
    {
        return match ($exitCode) {
            2 => new CorruptPdfError($stderr, $exitCode),
            3 => new EncryptionError($stderr, $exitCode),
            4 => new SourceUnreachableError($stderr, $exitCode),
            5 => new RemoteFetchInterruptedError($stderr, $exitCode),
            6 => new TlsError($stderr, $exitCode),
            10 => new ReceiptVerifyError($stderr, $exitCode),
            default => new PdftractException($stderr, $exitCode),
        };
    }

    /**
     * Convert source to CLI arguments.
     */
    private function sourceToArgs(string|\Stringable $source): array
    {
        if ($source instanceof \Stringable) {
            $source = (string) $source;
        }
        return [$source];
    }

    /**
     * Convert options array to CLI arguments (kebab-case).
     */
    private function optionsToArgs(array $options): array
    {
        $args = [];
        foreach ($options as $key => $value) {
            if ($value === null || $value === false) {
                continue;
            }

            // Convert camelCase to kebab-case
            $flag = preg_replace('/([A-Z])/', '-$1', lcfirst($key));
            $args[] = '--' . $flag;

            if ($value !== true) {
                $args[] = is_bool($value) ? ($value ? '1' : '0') : (string)$value;
            }
        }
        return $args;
    }

    /**
     * Extract full document structure.
     *
     * @param string|\Stringable $source Path or URL to PDF
     * @param array $options Options (ocrLanguage, ocrThreshold, preserveLayout, etc.)
     * @return Document Document structure
     */
    public function extract(string|\Stringable $source, array $options = []): Document
    {
        $args = array_merge(['extract', '--json'], $this->sourceToArgs($source));

        if (!empty($options)) {
            $args = array_merge($args, $this->optionsToArgs($options));
        }

        $result = $this->exec($args, true);
        return new Document($result);
    }

    /**
     * Extract plain text from document.
     *
     * @param string|\Stringable $source Path or URL to PDF
     * @param array $options Options (ocrLanguage, ocrThreshold, preserveLayout)
     * @return string Extracted text
     */
    public function extractText(string|\Stringable $source, array $options = []): string
    {
        $args = array_merge(['extract', '--text'], $this->sourceToArgs($source));

        if (!empty($options)) {
            $args = array_merge($args, $this->optionsToArgs($options));
        }

        return $this->exec($args, false);
    }

    /**
     * Extract Markdown from document.
     *
     * @param string|\Stringable $source Path or URL to PDF
     * @param array $options Options (ocrLanguage, ocrThreshold, preserveLayout)
     * @return string Extracted Markdown
     */
    public function extractMarkdown(string|\Stringable $source, array $options = []): string
    {
        $args = array_merge(['extract', '--md'], $this->sourceToArgs($source));

        if (!empty($options)) {
            $args = array_merge($args, $this->optionsToArgs($options));
        }

        return $this->exec($args, false);
    }

    /**
     * Stream pages one at a time (NDJSON format).
     *
     * @param string|\Stringable $source Path or URL to PDF
     * @param array $options Options (ocrLanguage, ocrThreshold, preserveLayout)
     * @return \Generator<Page> Generator yielding Page objects
     */
    public function extractStream(string|\Stringable $source, array $options = []): \Generator
    {
        $args = array_merge(['extract', '--ndjson'], $this->sourceToArgs($source));

        if (!empty($options)) {
            $args = array_merge($args, $this->optionsToArgs($options));
        }

        $cmd = escapeshellcmd($this->binaryPath);
        foreach ($args as $arg) {
            $cmd .= ' ' . escapeshellarg($arg);
        }

        $this->logger->debug("Streaming: {cmd}", ['cmd' => $cmd]);

        $descriptorspec = [
            0 => ['pipe', 'r'],
            1 => ['pipe', 'w'],
            2 => ['pipe', 'w'],
        ];

        $process = proc_open($cmd, $descriptorspec, $pipes);

        if (!is_resource($process)) {
            $error = "Failed to start process: $cmd";
            $this->logger->error($error);
            throw new PdftractException($error, -1);
        }

        fclose($pipes[0]);

        try {
            while (!feof($pipes[1])) {
                $line = fgets($pipes[1]);
                if ($line === false || trim($line) === '') {
                    continue;
                }

                $data = json_decode($line, true);
                if ($data !== null) {
                    yield new Page($data);
                }
            }

            $stderr = stream_get_contents($pipes[2]);
            fclose($pipes[1]);
            fclose($pipes[2]);

            $exitCode = proc_close($process);

            if ($exitCode !== 0) {
                $this->logger->error("Stream failed with exit code {code}", ['code' => $exitCode]);
                throw $this->mapError($stderr, $exitCode);
            }
        } catch (\Throwable $e) {
            // Ensure process is cleaned up on exception
            fclose($pipes[1]);
            fclose($pipes[2]);
            proc_close($process);
            throw $e;
        }
    }

    /**
     * Search for pattern in document.
     *
     * @param string|\Stringable $source Path or URL to PDF
     * @param string $pattern Search pattern
     * @param array $options Options (caseInsensitive, regex, wholeWord, maxResults)
     * @return \Generator<Match> Generator yielding Match objects
     */
    public function search(string|\Stringable $source, string $pattern, array $options = []): \Generator
    {
        $args = ['grep', $pattern, ...$this->sourceToArgs($source)];

        if (!empty($options)) {
            $args = array_merge($args, $this->optionsToArgs($options));
        }

        $cmd = escapeshellcmd($this->binaryPath);
        foreach ($args as $arg) {
            $cmd .= ' ' . escapeshellarg($arg);
        }

        $this->logger->debug("Searching: {cmd}", ['cmd' => $cmd]);

        $descriptorspec = [
            0 => ['pipe', 'r'],
            1 => ['pipe', 'w'],
            2 => ['pipe', 'w'],
        ];

        $process = proc_open($cmd, $descriptorspec, $pipes);

        if (!is_resource($process)) {
            $error = "Failed to start process: $cmd";
            $this->logger->error($error);
            throw new PdftractException($error, -1);
        }

        fclose($pipes[0]);

        try {
            while (!feof($pipes[1])) {
                $line = fgets($pipes[1]);
                if ($line === false || trim($line) === '') {
                    continue;
                }

                $data = json_decode($line, true);
                if ($data !== null) {
                    yield new Match($data);
                }
            }

            $stderr = stream_get_contents($pipes[2]);
            fclose($pipes[1]);
            fclose($pipes[2]);

            $exitCode = proc_close($process);

            if ($exitCode !== 0) {
                $this->logger->error("Search failed with exit code {code}", ['code' => $exitCode]);
                throw $this->mapError($stderr, $exitCode);
            }
        } catch (\Throwable $e) {
            // Ensure process is cleaned up on exception
            fclose($pipes[1]);
            fclose($pipes[2]);
            proc_close($process);
            throw $e;
        }
    }

    /**
     * Get document metadata.
     *
     * @param string|\Stringable $source Path or URL to PDF
     * @param array $options Options (timeout)
     * @return Metadata Document metadata
     */
    public function getMetadata(string|\Stringable $source, array $options = []): Metadata
    {
        $args = array_merge(['extract', '--metadata-only'], $this->sourceToArgs($source));

        if (!empty($options)) {
            $args = array_merge($args, $this->optionsToArgs($options));
        }

        $result = $this->exec($args, true);
        return new Metadata($result);
    }

    /**
     * Generate document fingerprint.
     *
     * @param string|\Stringable $source Path or URL to PDF
     * @param array $options Options (timeout)
     * @return Fingerprint Document fingerprint
     */
    public function hash(string|\Stringable $source, array $options = []): Fingerprint
    {
        $args = array_merge(['hash'], $this->sourceToArgs($source));

        if (!empty($options)) {
            $args = array_merge($args, $this->optionsToArgs($options));
        }

        $result = $this->exec($args, true);
        return new Fingerprint($result);
    }

    /**
     * Classify document type.
     *
     * @param string|\Stringable $source Path or URL to PDF
     * @return Classification Document classification
     */
    public function classify(string|\Stringable $source): Classification
    {
        $args = array_merge(['classify'], $this->sourceToArgs($source));
        $result = $this->exec($args, true);
        return new Classification($result);
    }

    /**
     * Verify receipt integrity.
     *
     * @param string $path Path to PDF file
     * @param Receipt $receipt Receipt object to verify
     * @return bool True if receipt is valid
     */
    public function verifyReceipt(string $path, Receipt $receipt): bool
    {
        // Receipt is passed as JSON string argument
        $receiptJson = json_encode($receipt);
        $output = $this->exec(['verify-receipt', $path, $receiptJson], false);
        return trim($output) === 'true';
    }
}
