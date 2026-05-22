package com.jedarden.pdftract;

import com.jedarden.pdftract.codegen.*;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Basic unit tests for the Pdftract client.
 */
public class PdftractTest {

    @Test
    @DisplayName("Pdftract client implements AutoCloseable")
    void testAutoCloseableInterface() {
        try (Pdftract client = new Pdftract()) {
            assertNotNull(client, "Client should be created");
        } // close() is called automatically
    }

    @Test
    @DisplayName("Client closes cleanly without subprocesses")
    void testCloseWithoutSubprocesses() {
        Pdftract client = new Pdftract();
        assertDoesNotThrow(() -> client.close(), "Close should not throw");
    }

    @Test
    @DisplayName("Source.fromPath creates PathSource")
    void testSourceFromPath() {
        Source source = Source.fromPath("/tmp/test.pdf");
        assertInstanceOf(PathSource.class, source);
        assertEquals(List.of("/tmp/test.pdf"), source.toArgs());
    }

    @Test
    @DisplayName("Source.fromUrl creates UrlSource")
    void testSourceFromUrl() {
        Source source = Source.fromUrl("https://example.com/doc.pdf");
        assertInstanceOf(UrlSource.class, source);
        assertEquals(List.of("https://example.com/doc.pdf"), source.toArgs());
    }

    @Test
    @DisplayName("Source.fromBytes creates BytesSource")
    void testSourceFromBytes(@TempDir Path tempDir) throws Exception {
        byte[] bytes = "fake pdf content".getBytes();
        Source source = Source.fromBytes(bytes);
        assertInstanceOf(BytesSource.class, source);

        List<String> args = source.toArgs();
        assertEquals(1, args.size());
        assertTrue(Files.exists(Path.of(args.get(0))), "Temp file should exist");
    }

    @Test
    @DisplayName("ExtractOptions builder pattern works")
    void testExtractOptionsBuilder() {
        ExtractOptions options = new ExtractOptions()
            .ocrLanguage("eng")
            .ocrThreshold(0.7)
            .password("secret");

        assertEquals("eng", options.ocrLanguage());
        assertEquals(0.7, options.ocrThreshold());
        assertEquals("secret", options.password());

        List<String> args = options.toArgs();
        assertTrue(args.contains("--ocr-language"));
        assertTrue(args.contains("eng"));
        assertTrue(args.contains("--ocr-threshold"));
        assertTrue(args.contains("0.7"));
        assertTrue(args.contains("--password"));
        assertTrue(args.contains("secret"));
    }

    @Test
    @DisplayName("SearchOptions builder pattern works")
    void testSearchOptionsBuilder() {
        SearchOptions options = new SearchOptions()
            .maxResults(100)
            .wholeWord(true)
            .password("secret");

        assertEquals(100, options.maxResults());
        assertEquals(true, options.wholeWord());
        assertEquals("secret", options.password());

        List<String> args = options.toArgs();
        assertTrue(args.contains("--max-results"));
        assertTrue(args.contains("100"));
        assertTrue(args.contains("--whole-word"));
    }

    @Test
    @DisplayName("BaseOptions builder pattern works")
    void testBaseOptionsBuilder() {
        BaseOptions options = new BaseOptions()
            .password("secret");

        assertEquals("secret", options.password());

        List<String> args = options.toArgs();
        assertTrue(args.contains("--password"));
        assertTrue(args.contains("secret"));
    }

    @Test
    @DisplayName("ExtractOptions can be empty")
    void testEmptyExtractOptions() {
        ExtractOptions options = new ExtractOptions();
        assertNull(options.ocrLanguage());
        assertNull(options.ocrThreshold());
        assertNull(options.password());
        assertTrue(options.toArgs().isEmpty());
    }

    @Test
    @DisplayName("SearchOptions can be empty")
    void testEmptySearchOptions() {
        SearchOptions options = new SearchOptions();
        assertNull(options.maxResults());
        assertNull(options.wholeWord());
        assertNull(options.password());
        assertTrue(options.toArgs().isEmpty());
    }

    @Test
    @DisplayName("Exception types are properly differentiated")
    void testExceptionTypes() {
        PdftractException base = new PdftractException("base", 1);
        CorruptPdfException corrupt = new CorruptPdfException("corrupt", 2);
        EncryptionException encrypt = new EncryptionException("encrypted", 3);
        SourceUnreachableException unreachable = new SourceUnreachableException("unreachable", 4);
        RemoteFetchInterruptedException remote = new RemoteFetchInterruptedException("remote", 5);
        TlsException tls = new TlsException("tls", 6);
        ReceiptVerifyException receipt = new ReceiptVerifyException("receipt", 10);

        assertTrue(base instanceof PdftractException);
        assertTrue(corrupt instanceof PdftractException);
        assertTrue(encrypt instanceof PdftractException);
        assertTrue(unreachable instanceof PdftractException);
        assertTrue(remote instanceof PdftractException);
        assertTrue(tls instanceof PdftractException);
        assertTrue(receipt instanceof PdftractException);

        assertEquals(1, base.getExitCode());
        assertEquals(2, corrupt.getExitCode());
        assertEquals(3, encrypt.getExitCode());
        assertEquals(4, unreachable.getExitCode());
        assertEquals(5, remote.getExitCode());
        assertEquals(6, tls.getExitCode());
        assertEquals(10, receipt.getExitCode());
    }

    @Test
    @DisplayName("Document record handles null values gracefully")
    void testDocumentRecordNullHandling() {
        Document doc = new Document(
            "1.0",
            null,
            null,
            null
        );

        assertEquals("1.0", doc.schemaVersion());
        assertNotNull(doc.metadata());
        assertNotNull(doc.pages());
        assertTrue(doc.pages().isEmpty());
        assertNotNull(doc.errors());
        assertTrue(doc.errors().isEmpty());
    }

    @Test
    @DisplayName("Page record handles null values gracefully")
    void testPageRecordNullHandling() {
        Page page = new Page(
            0,
            612.0,
            792.0,
            0,
            "vector",
            null,
            null
        );

        assertEquals(0, page.pageIndex());
        assertEquals("vector", page.pageType());
        assertNotNull(page.spans());
        assertTrue(page.spans().isEmpty());
        assertNotNull(page.blocks());
        assertTrue(page.blocks().isEmpty());
    }

    @Test
    @DisplayName("Classification record handles null labels")
    void testClassificationRecordNullHandling() {
        Classification cls = new Classification(
            "invoice",
            0.95,
            null
        );

        assertEquals("invoice", cls.category());
        assertEquals(0.95, cls.confidence());
        assertNotNull(cls.labels());
        assertTrue(cls.labels().isEmpty());
    }

    @Test
    @DisplayName("Source supports both Path and String")
    void testSourcePathVariants() {
        Source fromString = Source.fromPath("/tmp/test.pdf");
        Source fromPathObj = Source.fromPath(Path.of("/tmp/test.pdf"));

        assertInstanceOf(PathSource.class, fromString);
        assertInstanceOf(PathSource.class, fromPathObj);
        assertEquals(fromString.toArgs(), fromPathObj.toArgs());
    }

    @Test
    @DisplayName("Source URL supports both String and URI")
    void testSourceUrlVariants() {
        Source fromString = Source.fromUrl("https://example.com/doc.pdf");
        Source fromUri = Source.fromUrl(java.net.URI.create("https://example.com/doc.pdf"));

        assertInstanceOf(UrlSource.class, fromString);
        assertInstanceOf(UrlSource.class, fromUri);
        assertEquals(fromString.toArgs(), fromUri.toArgs());
    }

    @Test
    @DisplayName("Receipt record is properly structured")
    void testReceiptRecord() {
        Receipt receipt = new Receipt(
            "abc123",
            "sig456"
        );

        assertEquals("abc123", receipt.fingerprint());
        assertEquals("sig456", receipt.signature());
    }
}
