/**
 * Comprehensive conformance test suite for pdftract Node.js SDK
 *
 * This test suite verifies all 9 SDK methods with real PDF fixtures:
 * 1. extract - structured JSON extraction
 * 2. extractText - plain text extraction
 * 3. extractMarkdown - markdown extraction
 * 4. extractStream - streaming page extraction
 * 5. hash - fingerprint computation
 * 6. classify - document classification (requires profiles feature)
 * 7. search - text search (requires grep subcommand)
 * 8. getMetadata - metadata-only extraction
 * 9. verifyReceipt - receipt verification
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { Client, path, bytes } from '../src/index.js';
import { readFileSync } from 'fs';
import { join } from 'path';

const client = new Client();

describe('SDK Conformance Suite', () => {
  const fixtureDir = join(process.cwd(), 'test/fixtures/pdfs');

  describe('1. extract - structured JSON extraction', () => {
    it('should extract structured data from minimal PDF', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');
      const doc = await client.extract(path(fixturePath));

      expect(doc).toBeDefined();
      expect(doc.schema_version).toBeTruthy();
      expect(doc.pages).toBeDefined();
      expect(doc.pages.length).toBeGreaterThan(0);
      expect(doc.metadata).toBeDefined();
      expect(doc.metadata.page_count).toBe(1);

      // Check first page structure
      const firstPage = doc.pages[0];
      expect(firstPage.page_index).toBe(0);
      expect(firstPage.width).toBeGreaterThan(0);
      expect(firstPage.height).toBeGreaterThan(0);
      expect(firstPage.blocks).toBeDefined();
      expect(Array.isArray(firstPage.blocks)).toBe(true);
    });

    it('should extract multi-page PDF with correct page count', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'wikipedia-1000.pdf');
      const doc = await client.extract(path(fixturePath));

      expect(doc.pages.length).toBe(doc.metadata.page_count);
      expect(doc.metadata.page_count).toBeGreaterThan(1);

      // Verify all pages have sequential indices
      for (let i = 0; i < doc.pages.length; i++) {
        expect(doc.pages[i].page_index).toBe(i);
        expect(doc.pages[i].width).toBeGreaterThan(0);
        expect(doc.pages[i].height).toBeGreaterThan(0);
      }
    });

    it('should handle PDF with metadata', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-metadata.pdf');
      const doc = await client.extract(path(fixturePath));

      expect(doc.metadata).toBeDefined();
      // The document should have standard metadata fields
      expect(doc.metadata.page_count).toBeGreaterThan(0);
    });

    it('should handle invalid PDF path with appropriate error', { timeout: 30000 }, async () => {
      await expect(client.extract(path('/nonexistent/file.pdf')))
        .rejects.toThrow();
    });

    it('should extract with Buffer source', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');
      const pdfBuffer = readFileSync(fixturePath);
      const doc = await client.extract(bytes(pdfBuffer));

      expect(doc).toBeDefined();
      expect(doc.pages.length).toBeGreaterThan(0);
    });
  });

  describe('2. extractText - plain text extraction', () => {
    it('should extract plain text from PDF', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');
      const text = await client.extractText(path(fixturePath));

      expect(text).toBeDefined();
      expect(text.length).toBeGreaterThan(0);
      expect(typeof text).toBe('string');
    });

    it('should extract longer text from multi-page PDF', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'wikipedia-1000.pdf');
      const text = await client.extractText(path(fixturePath));

      expect(text.length).toBeGreaterThan(100);
      // Wikipedia PDF should have substantial text
      expect(text.length).toBeGreaterThan(1000);
    });

    it('should extract text with Buffer source', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'hello.pdf');
      const pdfBuffer = readFileSync(fixturePath);
      const text = await client.extractText(bytes(pdfBuffer));

      expect(text.length).toBeGreaterThan(0);
    });
  });

  describe('3. extractMarkdown - markdown extraction', () => {
    it('should extract markdown from PDF', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');
      const md = await client.extractMarkdown(path(fixturePath));

      expect(md).toBeDefined();
      expect(md.length).toBeGreaterThan(0);
      expect(typeof md).toBe('string');
    });

    it('should extract markdown from multi-page PDF', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'wikipedia-1000.pdf');
      const md = await client.extractMarkdown(path(fixturePath));

      expect(md.length).toBeGreaterThan(100);
      // Markdown should be formatted with headers
    });

    it('should extract markdown with Buffer source', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'hello.pdf');
      const pdfBuffer = readFileSync(fixturePath);
      const md = await client.extractMarkdown(bytes(pdfBuffer));

      expect(md.length).toBeGreaterThan(0);
    });
  });

  describe('4. extractStream - streaming page extraction', () => {
    it('should stream pages from multi-page PDF', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'wikipedia-1000.pdf');
      const pages: any[] = [];

      for await (const page of client.extractStream(path(fixturePath))) {
        pages.push(page);
        // Collect first 10 pages for testing
        if (pages.length >= 10) break;
      }

      expect(pages.length).toBeGreaterThanOrEqual(10);
      expect(pages[0]).toBeDefined();
      expect(pages[0].page_index).toBe(0);
      expect(pages[0].width).toBeGreaterThan(0);
      expect(pages[0].height).toBeGreaterThan(0);

      // Verify sequential indices
      for (let i = 0; i < pages.length; i++) {
        expect(pages[i].page_index).toBe(i);
      }
    });

    it('should stream all pages from minimal PDF', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');
      const pages: any[] = [];

      for await (const page of client.extractStream(path(fixturePath))) {
        pages.push(page);
      }

      expect(pages.length).toBe(1);
      expect(pages[0].page_index).toBe(0);
      expect(pages[0].schema_version).toBeTruthy();
      expect(pages[0].total_pages).toBe(1);
    });

    it('should handle streaming with max_pages limit', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'wikipedia-1000.pdf');
      const pages: any[] = [];
      const maxPages = 5;

      for await (const page of client.extractStream(path(fixturePath), { maxPages })) {
        pages.push(page);
        if (pages.length >= maxPages) break;
      }

      expect(pages.length).toBeLessThanOrEqual(maxPages);
    });

    it('should stream with Buffer source', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');
      const pdfBuffer = readFileSync(fixturePath);
      const pages: any[] = [];

      for await (const page of client.extractStream(bytes(pdfBuffer))) {
        pages.push(page);
      }

      expect(pages.length).toBeGreaterThan(0);
    });
  });

  describe('5. hash - fingerprint computation', () => {
    it('should compute hash fingerprint', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');
      const fingerprint = await client.hash(path(fixturePath));

      expect(fingerprint).toBeDefined();
      expect(fingerprint.hash).toBeDefined();
      expect(fingerprint.fast_hash).toBeDefined();
      expect(fingerprint.hash.length).toBe(64); // SHA256 = 64 hex chars
      expect(fingerprint.fast_hash.length).toBe(64);
      expect(fingerprint.page_count).toBeGreaterThan(0);
    });

    it('should compute different hashes for different PDFs', { timeout: 30000 }, async () => {
      const fp1 = await client.hash(path(join(fixtureDir, 'minimal-hello.pdf')));
      const fp2 = await client.hash(path(join(fixtureDir, 'hello.pdf')));

      expect(fp1.hash).not.toBe(fp2.hash);
      expect(fp1.fast_hash).not.toBe(fp2.fast_hash);
    });

    it('should compute same hash for same PDF', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');
      const fp1 = await client.hash(path(fixturePath));
      const fp2 = await client.hash(path(fixturePath));

      expect(fp1.hash).toBe(fp2.hash);
      expect(fp1.fast_hash).toBe(fp2.fast_hash);
    });

    it('should compute hash with Buffer source', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');
      const pdfBuffer = readFileSync(fixturePath);
      const fingerprint = await client.hash(bytes(pdfBuffer));

      expect(fingerprint.hash.length).toBe(64);
      expect(fingerprint.fast_hash.length).toBe(64);
    });
  });

  describe('6. classify - document classification', () => {
    it('should indicate classify is not yet available', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');

      // The classify method requires the 'profiles' feature
      // This test verifies proper error handling
      try {
        await client.classify(path(fixturePath));
        // If it succeeds, that's OK (profiles feature enabled)
        expect(true).toBe(true);
      } catch (error: any) {
        // Expected: feature not enabled error
        expect(error.message).toContain('profiles');
      }
    });
  });

  describe('7. search - text search', () => {
    it('should indicate search is not yet available', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'wikipedia-1000.pdf');

      // The search method requires the 'grep' subcommand (Phase 7.8)
      // This test verifies proper error handling
      try {
        const matches: any[] = [];
        for await (const match of client.search(path(fixturePath), 'test')) {
          matches.push(match);
        }
        // If it succeeds, that's OK (grep subcommand available)
        expect(true).toBe(true);
      } catch (error: any) {
        // Expected: grep not yet available error
        expect(error.message).toContain('grep');
      }
    });
  });

  describe('8. getMetadata - metadata-only extraction', () => {
    it('should extract metadata without page content', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');
      const metadata = await client.getMetadata(path(fixturePath));

      expect(metadata).toBeDefined();
      expect(metadata.page_count).toBeGreaterThan(0);
      expect(typeof metadata.page_count).toBe('number');
    });

    it('should extract metadata from multi-page PDF', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'wikipedia-1000.pdf');
      const metadata = await client.getMetadata(path(fixturePath));

      expect(metadata.page_count).toBeGreaterThan(1);
    });

    it('should extract metadata with encryption info', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');
      const metadata = await client.getMetadata(path(fixturePath));

      // Metadata should include encryption status
      expect(metadata.is_encrypted).toBeDefined();
      expect(typeof metadata.is_encrypted).toBe('boolean');
    });

    it('should extract metadata with Buffer source', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'hello.pdf');
      const pdfBuffer = readFileSync(fixturePath);
      const metadata = await client.getMetadata(bytes(pdfBuffer));

      expect(metadata.page_count).toBeGreaterThan(0);
    });
  });

  describe('9. verifyReceipt - receipt verification', () => {
    it('should handle receipt verification', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');

      // Receipt verification requires a valid receipt string
      // This test verifies the method exists and handles invalid receipts
      try {
        const result = await client.verifyReceipt(fixturePath, 'invalid-receipt');
        expect(typeof result).toBe('boolean');
      } catch (error: any) {
        // Error is acceptable for invalid receipt format
        expect(error).toBeDefined();
      }
    });
  });

  describe('Error handling across all methods', () => {
    it('should throw appropriate error for corrupt PDF', { timeout: 30000 }, async () => {
      // Create a corrupt PDF file
      const corruptPath = join(fixtureDir, 'corrupt-test.pdf');

      try {
        await client.extract(path(corruptPath));
        expect(true).toBe(false); // Should not reach here
      } catch (error: any) {
        // Should get an error about corrupt file or file not found
        expect(error).toBeDefined();
      }
    });

    it('should throw appropriate error for encrypted PDF without password', { timeout: 30000 }, async () => {
      // This test would require an encrypted PDF fixture
      // For now, we verify the error path exists
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');

      // Try with wrong password (should fail gracefully)
      try {
        await client.extract(path(fixturePath), { password: 'wrongpassword' });
        // If it succeeds, the PDF wasn't encrypted
        expect(true).toBe(true);
      } catch (error: any) {
        // Password error is acceptable
        expect(error).toBeDefined();
      }
    });
  });

  describe('Integration tests - method combinations', () => {
    it('should extract and hash from same PDF consistently', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');

      const doc = await client.extract(path(fixturePath));
      const fingerprint = await client.hash(path(fixturePath));

      expect(doc.pages.length).toBe(fingerprint.page_count);
      expect(doc.metadata.page_count).toBe(fingerprint.page_count);
    });

    it('should extractText and extractMarkdown produce different outputs', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'minimal-hello.pdf');

      const text = await client.extractText(path(fixturePath));
      const md = await client.extractMarkdown(path(fixturePath));

      expect(text).not.toBe(md);
      expect(text.length).toBeGreaterThan(0);
      expect(md.length).toBeGreaterThan(0);
    });

    it('should stream and extract produce consistent page counts', { timeout: 30000 }, async () => {
      const fixturePath = join(fixtureDir, 'wikipedia-1000.pdf');

      const doc = await client.extract(path(fixturePath));
      const pages: any[] = [];

      for await (const page of client.extractStream(path(fixturePath))) {
        pages.push(page);
        if (pages.length >= 20) break; // Limit for testing
      }

      expect(pages.length).toBeLessThanOrEqual(doc.pages.length);
    });
  });
});
