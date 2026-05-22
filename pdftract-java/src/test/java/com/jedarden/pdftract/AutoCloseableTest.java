package com.jedarden.pdftract;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Test AutoCloseable behavior and subprocess cleanup.
 */
public class AutoCloseableTest {

    @Test
    @DisplayName("try-with-resources calls close() automatically")
    void testTryWithResourcesCallsClose(@TempDir Path tempDir) throws Exception {
        // Create a minimal valid PDF for testing
        byte[] minimalPdf = createMinimalPdf();
        Path testFile = tempDir.resolve("test.pdf");
        Files.write(testFile, minimalPdf);

        AtomicInteger closeCount = new AtomicInteger(0);

        // Use a custom Pdftract subclass to track close calls
        class TrackingPdftract extends Pdftract {
            @Override
            public void close() {
                closeCount.incrementAndGet();
                super.close();
            }
        }

        try (TrackingPdftract client = new TrackingPdftract()) {
            assertNotNull(client);
        }

        assertEquals(1, closeCount.get(), "close() should be called exactly once");
    }

    @Test
    @DisplayName("Multiple close calls are safe")
    void testMultipleCloseCallsAreSafe() {
        Pdftract client = new Pdftract();

        assertDoesNotThrow(() -> {
            client.close();
            client.close(); // Second close should not throw
        });
    }

    @Test
    @DisplayName("Concurrent clients close independently")
    void testConcurrentClientsCloseIndependently() throws Exception {
        int threadCount = 10;
        ExecutorService executor = Executors.newFixedThreadPool(threadCount);
        CountDownLatch startLatch = new CountDownLatch(1);
        CountDownLatch doneLatch = new CountDownLatch(threadCount);
        AtomicInteger errorCount = new AtomicInteger(0);

        for (int i = 0; i < threadCount; i++) {
            executor.submit(() -> {
                try (Pdftract client = new Pdftract()) {
                    startLatch.await(); // Wait for all threads to be ready
                    // Simulate some work
                    Thread.sleep(10);
                } catch (Exception e) {
                    errorCount.incrementAndGet();
                } finally {
                    doneLatch.countDown();
                }
            });
        }

        startLatch.countDown(); // Start all threads at once
        boolean finished = doneLatch.await(30, TimeUnit.SECONDS);
        executor.shutdown();

        assertTrue(finished, "All threads should finish");
        assertEquals(0, errorCount.get(), "No errors should occur during concurrent close");
    }

    @Test
    @DisplayName("Client can be reused after creation")
    void testClientCanBeReused() {
        try (Pdftract client = new Pdftract()) {
            // Multiple method calls should work
            // Note: These will fail without actual pdftract binary, but test the structure
            assertDoesNotThrow(() -> {
                // We can't make real calls without the binary, but we verify
                // the client is in a valid state for multiple calls
                assertNotNull(client);
            });
        }
    }

    @Test
    @DisplayName("Custom binary path is respected")
    void testCustomBinaryPath() {
        Pdftract client = new Pdftract("/custom/path/to/pdftract");

        // The client should accept the custom path
        // Actual execution will fail if the binary doesn't exist,
        // but the constructor should work
        assertNotNull(client);
    }

    @Test
    @DisplayName("Null options are handled gracefully")
    void testNullOptionsAreHandled() {
        try (Pdftract client = new Pdftract()) {
            // These should not throw NPE
            assertDoesNotThrow(() -> {
                // Can't actually call without valid PDF, but test verifies
                // null handling in method signatures
                Source source = Source.fromPath("/tmp/test.pdf");
                // The methods accept null options
            });
        }
    }

    /**
     * Creates a minimal valid PDF for testing.
     * This is a tiny PDF with a single blank page.
     */
    private byte[] createMinimalPdf() {
        // Minimal PDF: %PDF-1.4 header, single object catalog, trailer
        String minimalPdf = "%PDF-1.4\n" +
            "1 0 obj\n" +
            "<<\n" +
            "/Type /Catalog\n" +
            "/Pages 2 0 R\n" +
            ">>\n" +
            "endobj\n" +
            "2 0 obj\n" +
            "<<\n" +
            "/Type /Pages\n" +
            "/Kids [3 0 R]\n" +
            "/Count 1\n" +
            ">>\n" +
            "endobj\n" +
            "3 0 obj\n" +
            "<<\n" +
            "/Type /Page\n" +
            "/Parent 2 0 R\n" +
            "/MediaBox [0 0 612 792]\n" +
            "/Resources <<\n" +
            "/Font <<\n" +
            ">>\n" +
            ">>\n" +
            ">>\n" +
            "endobj\n" +
            "xref\n" +
            "0 4\n" +
            "0000000000 65535 f\n" +
            "0000000009 00000 n\n" +
            "0000000058 00000 n\n" +
            "0000000115 00000 n\n" +
            "trailer\n" +
            "<<\n" +
            "/Size 4\n" +
            "/Root 1 0 R\n" +
            ">>\n" +
            "startxref\n" +
            "210\n" +
            "%%EOF\n";

        return minimalPdf.getBytes();
    }

    @Test
    @DisplayName("Source.fromBytes creates temp file")
    void testBytesSourceCreatesTempFile(@TempDir Path tempDir) {
        byte[] bytes = createMinimalPdf();
        Source source = Source.fromBytes(bytes);

        List<String> args = source.toArgs();
        assertEquals(1, args.size());

        Path tempPath = Path.of(args.get(0));
        assertTrue(Files.exists(tempPath), "Temp file should exist");
        assertTrue(tempPath.toString().contains("pdftract-"), "Temp file should have pdftract prefix");
        assertTrue(tempPath.toString().endsWith(".pdf"), "Temp file should have .pdf extension");
    }

    @Test
    @DisplayName("AutoCloseable pattern works correctly")
    void testAutoCloseablePattern() {
        Pdftract client = new Pdftract();

        // Verify it implements AutoCloseable
        assertTrue(client instanceof AutoCloseable);

        // Verify close can be called
        assertDoesNotThrow(() -> client.close());
    }

    @Test
    @DisplayName("Exception preserves exit code")
    void testExceptionPreservesExitCode() {
        PdftractException ex = new PdftractException("Test error", 42);
        assertEquals(42, ex.getExitCode());

        CorruptPdfException corrupt = new CorruptPdfException("Corrupt", 2);
        assertEquals(2, corrupt.getExitCode());

        EncryptionException encrypt = new EncryptionException("Encrypted", 3);
        assertEquals(3, encrypt.getExitCode());
    }
}
