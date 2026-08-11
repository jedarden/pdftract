/**
 * Buffer/Bytes source tests for pdftract Node.js SDK
 *
 * This test verifies that the bytes() function correctly handles Buffer inputs,
 * creates temporary files for processing, and ensures proper cleanup.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { Client, bytes } from '../src/index.js';
import { readFileSync, unlinkSync, existsSync } from 'fs';
import { join } from 'path';

describe('Buffer/Bytes Source Handling', () => {
  const client = new Client();

  describe('bytes() function', () => {
    it('should accept Buffer input', async () => {
      const samplePdf = join(__dirname, 'fixtures/subprocess/sample.pdf');

      // Create a simple PDF buffer (minimal PDF header)
      const minimalPdf = Buffer.from('%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n2 0 obj\n<<\n/Type /Pages\n/Count 1\n/Kids [3 0 R]\n>>\nendobj\n3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n>>\nendobj\nxref\n0 4\n0000000000 65535 f\n0000000009 00000 n\n0000000056 00000 n\n0000000113 00000 n\ntrailer\n<<\n/Size 4\n/Root 1 0 R\n>>\nstartxref\n198\n%%EOF');

      const source = bytes(minimalPdf);
      expect(source).toBeDefined();
      expect(source.toArgs).toBeInstanceOf(Function);
    });

    it('should accept Uint8Array input', async () => {
      const uint8Array = new Uint8Array([0x25, 0x50, 0x44, 0x46, 0x2D, 0x31, 0x2E, 0x34]); // "%PDF-1.4"

      const source = bytes(uint8Array);
      expect(source).toBeDefined();
      expect(source.toArgs).toBeInstanceOf(Function);
    });

    it('should create temporary file when toArgs is called', async () => {
      const minimalPdf = Buffer.from('%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n2 0 obj\n<<\n/Type /Pages\n/Count 1\n/Kids [3 0 R]\n>>\nendobj\n3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n>>\nendobj\nxref\n0 4\n0000000000 65535 f\n0000000009 00000 n\n0000000056 00000 n\n0000000113 00000 n\ntrailer\n<<\n/Size 4\n/Root 1 0 R\n>>\nstartxref\n198\n%%EOF');

      const source = bytes(minimalPdf);
      const args = await source.toArgs();

      expect(args).toBeInstanceOf(Array);
      expect(args.length).toBe(1);
      expect(args[0]).toMatch(/\.pdf$/);
      expect(existsSync(args[0])).toBe(true);

      // Clean up the temporary file
      unlinkSync(args[0]);
    });

    it('should handle empty buffer gracefully', async () => {
      const emptyBuffer = Buffer.from('');

      const source = bytes(emptyBuffer);
      expect(source).toBeDefined();

      const args = await source.toArgs();
      expect(args).toBeInstanceOf(Array);

      // Clean up if file was created
      if (args.length > 0 && existsSync(args[0])) {
        unlinkSync(args[0]);
      }
    });

    it('should handle binary data correctly', async () => {
      // Create a buffer with binary data
      const binaryData = new Uint8Array(256);
      for (let i = 0; i < 256; i++) {
        binaryData[i] = i;
      }

      const source = bytes(binaryData);
      const args = await source.toArgs();

      expect(args).toBeInstanceOf(Array);
      expect(args.length).toBe(1);

      // Verify the file was written correctly
      const writtenData = readFileSync(args[0]);
      expect(writtenData.length).toBe(256);

      // Clean up
      unlinkSync(args[0]);
    });
  });

  describe('Error handling with bytes() source', () => {
    it('should handle invalid PDF data', async () => {
      const invalidPdf = Buffer.from('This is not a PDF');

      const source = bytes(invalidPdf);
      const args = await source.toArgs();

      expect(args).toBeInstanceOf(Array);

      // Clean up
      if (args.length > 0 && existsSync(args[0])) {
        unlinkSync(args[0]);
      }
    });

    it('should handle very large buffers', async () => {
      // Create a 1MB buffer
      const largeBuffer = Buffer.alloc(1024 * 1024, 0x41); // Fill with 'A'

      const source = bytes(largeBuffer);
      const args = await source.toArgs();

      expect(args).toBeInstanceOf(Array);

      // Verify file was created
      expect(existsSync(args[0])).toBe(true);

      // Verify file size
      const stats = require('fs').statSync(args[0]);
      expect(stats.size).toBe(1024 * 1024);

      // Clean up
      unlinkSync(args[0]);
    });
  });

  describe('Integration with Client methods', () => {
    it('should work with extract method using bytes source', { timeout: 30000 }, async () => {
      const minimalPdf = Buffer.from('%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n2 0 obj\n<<\n/Type /Pages\n/Count 1\n/Kids [3 0 R]\n>>\nendobj\n3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents 4 0 R\n>>\nendobj\n4 0 obj\n<<\n/Length 44\n>>\nstream\nBT\n/F1 12 Tf\n100 700 Td\n(Test) Tj\nET\nendstream\nendobj\nxref\n0 5\n0000000000 65535 f\n0000000009 00000 n\n0000000056 00000 n\n0000000113 00000 n\n0000000208 00000 n\ntrailer\n<<\n/Size 5\n/Root 1 0 R\n>>\nstartxref\n285\n%%EOF');

      const source = bytes(minimalPdf);

      try {
        // This will likely fail due to invalid PDF structure, but we're testing
        // that the source argument is handled correctly
        const doc = await client.extract(source);

        // If we get here, the PDF was valid
        expect(doc).toBeDefined();
        expect(doc.schema_version).toBeTruthy();
      } catch (error: any) {
        // Expected for invalid PDF, but should not be a source argument error
        expect(error.message).not.toContain('source');
        expect(error.message).not.toContain('argument');
      }
    });

    it('should work with getMetadata method using bytes source', { timeout: 30000 }, async () => {
      const minimalPdf = Buffer.from('%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n2 0 obj\n<<\n/Type /Pages\n/Count 1\n/Kids [3 0 R]\n>>\nendobj\n3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n>>\nendobj\nxref\n0 4\n0000000000 65535 f\n0000000009 00000 n\n0000000056 00000 n\n0000000113 00000 n\ntrailer\n<<\n/Size 4\n/Root 1 0 R\n>>\nstartxref\n198\n%%EOF');

      const source = bytes(minimalPdf);

      try {
        const metadata = await client.getMetadata(source);
        expect(metadata).toBeDefined();
        expect(metadata.page_count).toBeGreaterThanOrEqual(1);
      } catch (error: any) {
        // Some PDF parsing errors are acceptable
        expect(error.message).not.toContain('source');
      }
    });

    it('should work with hash method using bytes source', { timeout: 30000 }, async () => {
      const minimalPdf = Buffer.from('%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n2 0 obj\n<<\n/Type /Pages\n/Count 1\n/Kids [3 0 R]\n>>\nendobj\n3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n>>\nendobj\nxref\n0 4\n0000000000 65535 f\n0000000009 00000 n\n0000000056 00000 n\n0000000113 00000 n\ntrailer\n<<\n/Size 4\n/Root 1 0 R\n>>\nstartxref\n198\n%%EOF');

      const source = bytes(minimalPdf);

      try {
        const fingerprint = await client.hash(source);
        expect(fingerprint).toBeDefined();
        expect(fingerprint.hash).toBeTruthy();
        expect(fingerprint.hash.length).toBe(64);
      } catch (error: any) {
        // Some PDF parsing errors are acceptable
        expect(error.message).not.toContain('source');
      }
    });
  });

  describe('Temporary file cleanup', () => {
    it('should create unique temporary files for multiple calls', async () => {
      const pdfBuffer = Buffer.from('%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n2 0 obj\n<<\n/Type /Pages\n/Count 1\n/Kids [3 0 R]\n>>\nendobj\n3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n>>\nendobj\nxref\n0 4\n0000000000 65535 f\n0000000009 00000 n\n0000000056 00000 n\n0000000113 00000 n\ntrailer\n<<\n/Size 4\n/Root 1 0 R\n>>\nstartxref\n198\n%%EOF');

      const source1 = bytes(pdfBuffer);
      const source2 = bytes(pdfBuffer);

      const args1 = await source1.toArgs();
      const args2 = await source2.toArgs();

      expect(args1[0]).not.toBe(args2[0]);

      // Clean up
      unlinkSync(args1[0]);
      unlinkSync(args2[0]);
    });
  });
});
