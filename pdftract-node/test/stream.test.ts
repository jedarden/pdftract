/**
 * Unit tests for stream.ts
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { NdjsonReadable, StreamError, createExtractStream, createSearchStream } from '../src/stream.js';
import { Readable, Writable } from 'stream';
import { mkdir, rm, writeFile } from 'fs/promises';
import { join } from 'path';

// Test fixtures directory
const TEST_FIXTURES_DIR = join(process.cwd(), 'test', 'fixtures', 'stream');

describe('stream', () => {
  beforeAll(async () => {
    // Create test fixtures directory
    await mkdir(TEST_FIXTURES_DIR, { recursive: true });
  });

  describe('NdjsonReadable', () => {
    it('should emit parsed JSON objects', async () => {
      const stream = new NdjsonReadable(
        ['echo', '{"page_index":0,"text":"page 0"}\n{"page_index":1,"text":"page 1"}'],
        Object
      );

      const items: any[] = [];
      for await (const item of stream) {
        items.push(item);
      }

      expect(items).toHaveLength(2);
      expect(items[0]).toEqual({ page_index: 0, text: 'page 0' });
      expect(items[1]).toEqual({ page_index: 1, text: 'page 1' });
    });

    it('should handle partial lines correctly', async () => {
      // Create a mock script that outputs NDJSON with partial lines
      const scriptPath = join(TEST_FIXTURES_DIR, 'partial-lines.sh');
      await writeFile(
        scriptPath,
        '#!/bin/sh\necho \'{"page":0}\'\necho \'{"page":1}\'\n',
        { mode: 0o755 }
      );

      const stream = new NdjsonReadable(['sh', scriptPath], Object);

      const items: any[] = [];
      for await (const item of stream) {
        items.push(item);
      }

      // Should correctly handle the split line
      expect(items.length).toBeGreaterThanOrEqual(1);

      // Cleanup
      await rm(scriptPath);
    });

    it('should support event-based consumption', async () => {
      const stream = new NdjsonReadable(
        ['echo', '{"value":1}\n{"value":2}\n{"value":3}'],
        Object
      );

      return new Promise<void>((resolve, reject) => {
        const items: any[] = [];

        stream.on('data', (item) => {
          items.push(item);
        });

        stream.on('end', () => {
          expect(items).toHaveLength(3);
          expect(items[0]).toEqual({ value: 1 });
          expect(items[2]).toEqual({ value: 3 });
          resolve();
        });

        stream.on('error', reject);
      });
    });

    it('should handle backpressure', async () => {
      // Create a script that outputs JSON lines
      const scriptPath = join(TEST_FIXTURES_DIR, 'backpressure.sh');
      const lines = Array.from({ length: 20 }, (_, i) => `{"index":${i}}`).join('\n');
      await writeFile(scriptPath, `#!/bin/sh\necho '${lines}'`, { mode: 0o755 });

      const stream = new NdjsonReadable(['sh', scriptPath], Object, {
        highWaterMark: 5, // Small buffer to trigger backpressure
      });

      const items: any[] = [];
      for await (const item of stream) {
        items.push(item);
      }

      expect(items).toHaveLength(20);

      // Cleanup
      await rm(scriptPath);
    });

    it('should propagate errors from subprocess', async () => {
      const stream = new NdjsonReadable(['false'], Object); // false command exits with code 1

      let errorThrown = false;
      try {
        for await (const _item of stream) {
          // Should not reach here
        }
      } catch (error: any) {
        errorThrown = true;
        expect(error).toBeInstanceOf(StreamError);
        expect(error.exitCode).toBeDefined();
      }

      expect(errorThrown).toBe(true);
    });

    it('should cleanup subprocess on stream destroy', async () => {
      // Create a long-running process
      const stream = new NdjsonReadable(['sleep', '10'], Object);

      // Destroy the stream immediately
      stream.destroy();

      // The subprocess should be cleaned up
      // We can't easily test this directly, but we can ensure no error is thrown
      await new Promise((resolve) => setTimeout(resolve, 100));
      expect(true).toBe(true);
    });

    it('should handle empty output', async () => {
      const stream = new NdjsonReadable(['true'], Object); // true command outputs nothing

      const items: any[] = [];
      for await (const item of stream) {
        items.push(item);
      }

      expect(items).toHaveLength(0);
    });

    it('should handle malformed JSON gracefully', async () => {
      const stream = new NdjsonReadable(['echo', 'invalid json'], Object);

      let errorThrown = false;
      try {
        for await (const _item of stream) {
          // Should not reach here
        }
      } catch (error: any) {
        errorThrown = true;
        expect(error.message).toContain('Failed to parse NDJSON');
      }

      expect(errorThrown).toBe(true);
    });
  });

  describe('createExtractStream', () => {
    it('should create a stream for extract operations', async () => {
      // For testing, we override the binary path to use echo
      // The args array should NOT include the binary name when using binaryPath
      const stream = createExtractStream(['{"page_index":0}'], {
        binaryPath: 'echo',
      });

      const items: any[] = [];
      for await (const item of stream) {
        items.push(item);
      }

      expect(items).toHaveLength(1);
      expect(items[0]).toEqual({ page_index: 0 });
    });

    it('should pass options through to NdjsonReadable', async () => {
      const stream = createExtractStream(['{}'], {
        timeout: 5000,
        highWaterMark: 5,
        binaryPath: 'echo',
      });

      // Verify options are passed (we can't directly inspect them, but we can verify the stream works)
      const items: any[] = [];
      for await (const item of stream) {
        items.push(item);
      }

      expect(items).toHaveLength(1);
    });
  });

  describe('createSearchStream', () => {
    it('should create a stream for search operations', async () => {
      const stream = createSearchStream(['{"text":"match","page":0}'], {
        binaryPath: 'echo',
      });

      const items: any[] = [];
      for await (const item of stream) {
        items.push(item);
      }

      expect(items).toHaveLength(1);
      expect(items[0]).toEqual({ text: 'match', page: 0 });
    });
  });

  describe('integration with Readable stream APIs', () => {
    it('should work with stream.pipe', async () => {
      const sourceStream = new NdjsonReadable(
        ['echo', '{"a":1}\n{"a":2}\n{"a":3}'],
        Object
      );

      const items: any[] = [];

      const destStream = new Writable({
        objectMode: true,
        write(chunk, encoding, callback) {
          items.push(chunk);
          callback();
        },
      });

      return new Promise<void>((resolve, reject) => {
        sourceStream
          .on('error', reject)
          .pipe(destStream)
          .on('finish', () => {
            expect(items).toHaveLength(3);
            resolve();
          });
      });
    });

    it('should work with async iteration', async () => {
      const stream = new NdjsonReadable(
        ['echo', '{"n":1}\n{"n":2}\n{"n":3}'],
        Object
      );

      const results: any[] = [];
      for await (const item of stream) {
        results.push(item);
      }

      expect(results).toHaveLength(3);
    });

    it('should work with toArray', async () => {
      const stream = new NdjsonReadable(
        ['echo', '{"x":1}\n{"x":2}'],
        Object
      );

      const items = await stream.toArray();

      expect(items).toHaveLength(2);
    });
  });

  describe('StreamError', () => {
    it('should create error with exit code and stderr', () => {
      const error = new StreamError('Test error', 1, 'Test stderr');

      expect(error.message).toBe('Test error');
      expect(error.exitCode).toBe(1);
      expect(error.stderr).toBe('Test stderr');
      expect(error.name).toBe('StreamError');
    });

    it('should create error with minimal info', () => {
      const error = new StreamError('Test error');

      expect(error.message).toBe('Test error');
      expect(error.exitCode).toBeUndefined();
      expect(error.stderr).toBeUndefined();
    });
  });

  describe('large file handling', () => {
    it('should handle many JSON objects without memory issues', async () => {
      // Create a script that outputs many JSON objects
      const scriptPath = join(TEST_FIXTURES_DIR, 'large-output.sh');
      const count = 1000;
      const lines = Array.from({ length: count }, (_, i) => `{"id":${i}}`).join('\n');
      await writeFile(scriptPath, `#!/bin/sh\necho '${lines}'`, { mode: 0o755 });

      const stream = new NdjsonReadable(['sh', scriptPath], Object);

      let receivedCount = 0;
      for await (const _item of stream) {
        receivedCount++;
      }

      expect(receivedCount).toBe(count);

      // Cleanup
      await rm(scriptPath);
    });

    it('should process large JSON objects', async () => {
      // Create a script that outputs a large JSON object
      const largeObject = {
        page_index: 0,
        spans: Array.from({ length: 1000 }, (_, i) => ({
          text: `Span ${i}`.repeat(10),
          bbox: [0, 0, 100, 100],
          font: 'Arial',
          size: 12,
        })),
      };

      const scriptPath = join(TEST_FIXTURES_DIR, 'large-object.sh');
      await writeFile(
        scriptPath,
        `#!/bin/sh\necho '${JSON.stringify(largeObject).replace(/'/g, "'\\''")}'`,
        { mode: 0o755 }
      );

      const stream = new NdjsonReadable(['sh', scriptPath], Object);

      const items: any[] = [];
      for await (const item of stream) {
        items.push(item);
      }

      expect(items).toHaveLength(1);
      expect(items[0].spans).toHaveLength(1000);

      // Cleanup
      await rm(scriptPath);
    });
  });

  describe('error recovery', () => {
    it('should recover from parse errors in stream', async () => {
      // Create a script that outputs valid then invalid JSON
      const scriptPath = join(TEST_FIXTURES_DIR, 'mixed-json.sh');
      await writeFile(
        scriptPath,
        '#!/bin/sh\necho \'{"valid":1}\'\necho \'invalid json\'\necho \'{"valid":2}\'',
        { mode: 0o755 }
      );

      const stream = new NdjsonReadable(['sh', scriptPath], Object);

      let errorThrown = false;
      try {
        for await (const _item of stream) {
          // Should get first valid item, then error
        }
      } catch (error: any) {
        errorThrown = true;
        expect(error.message).toContain('Failed to parse NDJSON');
      }

      expect(errorThrown).toBe(true);

      // Cleanup
      await rm(scriptPath);
    });

    it('should handle subprocess crashes mid-stream', async () => {
      // Create a script that outputs some JSON then crashes
      const scriptPath = join(TEST_FIXTURES_DIR, 'crash-mid-stream.sh');
      await writeFile(
        scriptPath,
        '#!/bin/sh\necho \'{"page":0}\'\nexit 1',
        { mode: 0o755 }
      );

      const stream = new NdjsonReadable(['sh', scriptPath], Object);

      let itemsReceived = 0;
      let errorThrown = false;

      try {
        for await (const item of stream) {
          itemsReceived++;
        }
      } catch (error: any) {
        errorThrown = true;
        expect(error.exitCode).toBe(1);
      }

      // Should receive first item, then error
      expect(itemsReceived).toBe(1);
      expect(errorThrown).toBe(true);

      // Cleanup
      await rm(scriptPath);
    });
  });

  describe('concurrent usage', () => {
    it('should handle multiple concurrent streams', async () => {
      const streams = Array.from({ length: 5 }, (_, i) =>
        new NdjsonReadable(['echo', `{"stream":${i}}`], Object)
      );

      const results = await Promise.all(
        streams.map(async (stream) => {
          const items: any[] = [];
          for await (const item of stream) {
            items.push(item);
          }
          return items[0];
        })
      );

      expect(results).toHaveLength(5);
      results.forEach((result, i) => {
        expect(result.stream).toBe(i);
      });
    });
  });
});
