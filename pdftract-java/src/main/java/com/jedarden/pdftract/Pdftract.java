package com.jedarden.pdftract;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.jedarden.pdftract.codegen.*;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.stream.Stream;

/**
 * Main pdftract client.
 * AutoCloseable - use with try-with-resources.
 *
 * <p>This is the primary entry point for the pdftract SDK.
 * Each method invocation spawns a subprocess to execute the pdftract binary.</p>
 *
 * <p>Example usage:</p>
 * <pre>{@code
 * try (Pdftract client = new Pdftract()) {
 *     Document doc = client.extract(Source.fromPath("document.pdf"), null);
 *     System.out.println("Pages: " + doc.pages().size());
 * }
 * }</pre>
 */
public class Pdftract implements AutoCloseable {
    private final String binaryPath;
    private final String version;
    private final ObjectMapper mapper;
    private final List<Process> childProcesses = new ArrayList<>();

    /**
     * Creates a new Pdftract client using the default binary name "pdftract".
     * The binary must be available on the PATH.
     */
    public Pdftract() {
        this("pdftract");
    }

    /**
     * Creates a new Pdftract client using a specific binary path.
     *
     * @param binaryPath Path to the pdftract binary
     */
    public Pdftract(String binaryPath) {
        this.binaryPath = binaryPath;
        this.version = "0.1.0";
        this.mapper = com.jedarden.pdftract.codegen.Json.mapper();
    }

    /**
     * Extract structured data from a PDF.
     *
     * @param source The PDF source (file path, URL, or bytes)
     * @param options Extraction options (can be null for defaults)
     * @return Extracted document with pages, blocks, and spans
     * @throws PdftractException on extraction errors
     */
    public Document extract(Source source, ExtractOptions options) throws PdftractException {
        List<String> args = new ArrayList<>();
        args.add("extract");
        args.addAll(source.toArgs());

        if (options != null) {
            args.addAll(options.toArgs());
        }

        ProcessResult result = exec(args.toArray(new String[0]));
        return parseJson(result.stdout(), Document.class);
    }

    /**
     * Extract plain text from a PDF.
     *
     * @param source The PDF source
     * @param options Extraction options
     * @return Extracted plain text
     * @throws PdftractException on extraction errors
     */
    public String extractText(Source source, ExtractOptions options) throws PdftractException {
        List<String> args = new ArrayList<>();
        args.add("extract");
        args.addAll(source.toArgs());

        if (options != null) {
            args.addAll(options.toArgs());
        }

        args.add("--text");

        ProcessResult result = exec(args.toArray(new String[0]));
        return result.stdout().trim();
    }

    /**
     * Extract Markdown-formatted text from a PDF.
     *
     * @param source The PDF source
     * @param options Extraction options
     * @return Extracted Markdown text
     * @throws PdftractException on extraction errors
     */
    public String extractMarkdown(Source source, ExtractOptions options) throws PdftractException {
        List<String> args = new ArrayList<>();
        args.add("extract");
        args.addAll(source.toArgs());

        if (options != null) {
            args.addAll(options.toArgs());
        }

        args.add("--md");

        ProcessResult result = exec(args.toArray(new String[0]));
        return result.stdout().trim();
    }

    /**
     * Extract pages from a PDF as a stream.
     * Each page is emitted as it's parsed from the subprocess NDJSON output.
     *
     * <p>The subprocess runs on a background daemon thread and is killed when
     * the stream is closed or exhausted.</p>
     *
     * @param source The PDF source
     * @param options Extraction options
     * @return Stream of pages
     * @throws PdftractException on extraction errors
     */
    public Stream<Page> extractStream(Source source, ExtractOptions options) throws PdftractException {
        List<String> args = new ArrayList<>();
        args.add("extract");
        args.addAll(source.toArgs());

        if (options != null) {
            args.addAll(options.toArgs());
        }

        return streamNdjson(args, Page.class);
    }

    /**
     * Search for text patterns in a PDF.
     *
     * <p>Returns a stream of matches. The subprocess runs on a background
     * daemon thread and is killed when the stream is closed or exhausted.</p>
     *
     * @param source The PDF source
     * @param pattern The search pattern (regex supported)
     * @param options Search options
     * @return Stream of matches
     * @throws PdftractException on search errors
     */
    public Stream<Match> search(Source source, String pattern, SearchOptions options) throws PdftractException {
        List<String> args = new ArrayList<>();
        args.add("grep");
        args.add(pattern);
        args.addAll(source.toArgs());

        if (options != null) {
            args.addAll(options.toArgs());
        }

        return streamNdjson(args, Match.class);
    }

    /**
     * Get metadata from a PDF.
     *
     * @param source The PDF source
     * @param options Base options
     * @return PDF metadata
     * @throws PdftractException on errors
     */
    public Metadata getMetadata(Source source, BaseOptions options) throws PdftractException {
        List<String> args = new ArrayList<>();
        args.add("extract");
        args.addAll(source.toArgs());

        if (options != null) {
            args.addAll(options.toArgs());
        }

        args.add("--metadata-only");

        ProcessResult result = exec(args.toArray(new String[0]));
        return parseJson(result.stdout(), Metadata.class);
    }

    /**
     * Compute hash fingerprint of a PDF.
     *
     * @param source The PDF source
     * @param options Base options
     * @return Fingerprint with SHA-256 hash
     * @throws PdftractException on errors
     */
    public Fingerprint hash(Source source, BaseOptions options) throws PdftractException {
        List<String> args = new ArrayList<>();
        args.add("hash");
        args.addAll(source.toArgs());

        if (options != null) {
            args.addAll(options.toArgs());
        }

        ProcessResult result = exec(args.toArray(new String[0]));
        return parseJson(result.stdout(), Fingerprint.class);
    }

    /**
     * Classify a PDF document.
     *
     * @param source The PDF source
     * @return Classification with category and confidence
     * @throws PdftractException on errors
     */
    public Classification classify(Source source) throws PdftractException {
        List<String> args = new ArrayList<>();
        args.add("classify");
        args.addAll(source.toArgs());

        ProcessResult result = exec(args.toArray(new String[0]));
        return parseJson(result.stdout(), Classification.class);
    }

    /**
     * Verify a receipt signature.
     *
     * @param path Path to the receipt PDF
     * @param receipt Receipt data with fingerprint and signature
     * @return true if receipt is valid, false otherwise
     * @throws PdftractException on verification errors
     */
    public boolean verifyReceipt(Path path, Receipt receipt) throws PdftractException {
        List<String> args = new ArrayList<>();
        args.add("verify-receipt");
        args.add(path.toString());

        // Serialize receipt as JSON
        String receiptJson;
        try {
            receiptJson = mapper.writeValueAsString(receipt);
        } catch (IOException e) {
            throw new PdftractException("Failed to serialize receipt", -1, e.getMessage());
        }
        args.add(receiptJson);

        ProcessResult result = exec(args.toArray(new String[0]));
        return Boolean.parseBoolean(result.stdout().trim());
    }

    /**
     * Closes this client and terminates any running child processes.
     * This method is automatically called when used with try-with-resources.
     */
    @Override
    public void close() {
        synchronized (childProcesses) {
            for (Process process : childProcesses) {
                if (process.isAlive()) {
                    process.destroyForcibly();
                }
            }
            childProcesses.clear();
        }
    }

    /**
     * Execute a subprocess and capture output.
     */
    private ProcessResult exec(String... args) throws PdftractException {
        try {
            ProcessBuilder pb = new ProcessBuilder(binaryPath);
            pb.command().addAll(List.of(args));
            pb.redirectErrorStream(true);

            Process process = pb.start();
            childProcesses.add(process);

            StringBuilder stdout = new StringBuilder();
            try (BufferedReader reader = new BufferedReader(new InputStreamReader(process.getInputStream()))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    stdout.append(line).append("\n");
                }
            }

            int exitCode = process.waitFor();
            childProcesses.remove(process);

            String output = stdout.toString();

            if (exitCode != 0) {
                throw mapError(output, exitCode);
            }

            return new ProcessResult(output, exitCode);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new PdftractException("Interrupted", -1, e.getMessage());
        } catch (IOException e) {
            throw new PdftractException("IO error", -1, e.getMessage());
        }
    }

    /**
     * Stream NDJSON output from a subprocess.
     * Each line is parsed as a JSON object.
     */
    private <T> Stream<T> streamNdjson(List<String> args, Class<T> clazz) throws PdftractException {
        try {
            ProcessBuilder pb = new ProcessBuilder(binaryPath);
            pb.command(args);
            pb.redirectErrorStream(true);

            Process process = pb.start();
            childProcesses.add(process);

            InputStream inputStream = process.getInputStream();
            BufferedReader reader = new BufferedReader(new InputStreamReader(inputStream));

            AtomicBoolean closed = new AtomicBoolean(false);

            Stream<T> stream = Stream.<T>generate(() -> {
                try {
                    String line = reader.readLine();
                    if (line == null) {
                        return null;
                    }
                    return mapper.readValue(line, clazz);
                } catch (IOException e) {
                    throw new RuntimeException("Failed to parse NDJSON line", e);
                }
            })
            .takeWhile(item -> item != null)
            .onClose(() -> {
                if (closed.compareAndSet(false, true)) {
                    try {
                        reader.close();
                    } catch (IOException e) {
                        // Ignore
                    }
                    if (process.isAlive()) {
                        process.destroyForcibly();
                    }
                    childProcesses.remove(process);
                }
            });

            return stream;
        } catch (IOException e) {
            throw new PdftractException("Failed to start subprocess", -1, e.getMessage());
        }
    }

    /**
     * Map exit codes to specific exception types.
     */
    private PdftractException mapError(String stderr, int exitCode) {
        return switch (exitCode) {
            case 2 -> new CorruptPdfException(stderr, exitCode);
            case 3 -> new EncryptionException(stderr, exitCode);
            case 4 -> new SourceUnreachableException(stderr, exitCode);
            case 5 -> new RemoteFetchInterruptedException(stderr, exitCode);
            case 6 -> new TlsException(stderr, exitCode);
            case 10 -> new ReceiptVerifyException(stderr, exitCode);
            default -> new PdftractException(stderr, exitCode);
        };
    }

    /**
     * Parse JSON string to object.
     */
    private <T> T parseJson(String json, Class<T> clazz) throws PdftractException {
        try {
            return mapper.readValue(json, clazz);
        } catch (IOException e) {
            throw new PdftractException("Failed to parse JSON response", -1, e.getMessage());
        }
    }

    private record ProcessResult(String stdout, int exitCode) {}
}
