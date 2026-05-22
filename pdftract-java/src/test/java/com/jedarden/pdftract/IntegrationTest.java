package com.jedarden.pdftract;

import com.jedarden.pdftract.*;
import com.jedarden.pdftract.codegen.*;
import java.nio.file.Files;
import java.nio.file.Path;

/**
 * Quick integration test to verify the SDK works with the actual pdftract binary.
 */
public class IntegrationTest {
    public static void main(String[] args) throws Exception {
        System.out.println("=== pdftract Java SDK Integration Test ===\n");

        // Find a test fixture
        String fixturePath = "/home/coding/pdftract/tests/sdk-conformance/fixtures/contract/invoice.pdf";
        if (!Files.exists(Path.of(fixturePath))) {
            System.err.println("Test fixture not found: " + fixturePath);
            System.err.println("Skipping integration test - run from pdftract repo with test fixtures");
            return;
        }

        try (Pdftract client = new Pdftract()) {
            System.out.println("1. Testing extract()...");
            Document doc = client.extract(Source.fromPath(fixturePath), null);
            System.out.println("   ✓ Extracted document with " + doc.pages().size() + " page(s)");
            System.out.println("   Schema version: " + doc.schemaVersion());
            System.out.println("   Page count (metadata): " + doc.metadata().pageCount());

            System.out.println("\n2. Testing extractText()...");
            String text = client.extractText(Source.fromPath(fixturePath), null);
            System.out.println("   ✓ Extracted " + text.length() + " characters of text");

            System.out.println("\n3. Testing getMetadata()...");
            Metadata metadata = client.getMetadata(Source.fromPath(fixturePath), null);
            System.out.println("   ✓ Metadata - page count: " + metadata.pageCount());

            System.out.println("\n4. Testing hash()...");
            Fingerprint fp = client.hash(Source.fromPath(fixturePath), null);
            System.out.println("   ✓ Hash: " + fp.hash().substring(0, 16) + "...");
            System.out.println("   ✓ Page count: " + fp.pageCount());

            System.out.println("\n5. Testing classify()...");
            Classification cls = client.classify(Source.fromPath(fixturePath));
            System.out.println("   ✓ Category: " + cls.category());
            System.out.println("   ✓ Confidence: " + cls.confidence());

            System.out.println("\n6. Testing search()...");
            long matchCount = client.search(Source.fromPath(fixturePath), "invoice", null).count();
            System.out.println("   ✓ Found " + matchCount + " matches for 'invoice'");

            System.out.println("\n7. Testing extractStream()...");
            long pageCount = client.extractStream(Source.fromPath(fixturePath), null).count();
            System.out.println("   ✓ Streamed " + pageCount + " page(s)");

            System.out.println("\n=== All integration tests passed! ===");
        } catch (PdftractException e) {
            System.err.println("✗ PdftractException: " + e.getMessage());
            System.err.println("  Exit code: " + e.getExitCode());
            System.exit(1);
        }
    }
}
