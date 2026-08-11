/**
 * This file is auto-generated. Do not edit manually.
 */

import { spawnPdftract, spawnPdftractStream } from '../subprocess.js';
import { NdjsonReadable, createExtractStream, createSearchStream } from '../stream.js';
import type { NdjsonReadableOptions } from '../stream.js';
import {
  PathSource,
  URLSource,
  BytesSource,
} from './types.js';
import type {
  Source,
  Document,
  Page,
  Match,
  Fingerprint,
  Classification,
  Metadata,
  ExtractOptions,
  SearchOptions,
  BaseOptions
} from './types.js';
import {
  PdftractError,
  CorruptPdfError,
  EncryptionError,
  SourceUnreachableError,
  RemoteFetchInterruptedError,
  TlsError,
  ReceiptVerifyError,
  ValidationError
} from './errors.js';
import { normalizeOptions, validateOptions } from '../ergonomics.js';

/**
 * Maps exit codes to error classes.
 */
const ERROR_MAP: Record<number, typeof PdftractError> = {
  1: ValidationError,
  2: CorruptPdfError,
  3: EncryptionError,
  4: SourceUnreachableError,
  5: RemoteFetchInterruptedError,
  6: TlsError,
  10: ReceiptVerifyError,
};

/**
 * Main SDK client for pdftract.
 */
export class Client {
  private binaryPath: string;
  private version: string;

  constructor(binaryPath: string = 'pdftract') {
    this.binaryPath = binaryPath;
    this.version = '1.0.0';
  }

  private mapError(stderr: string, exitCode: number): PdftractError {
    const ErrorClass = ERROR_MAP[exitCode];
    if (ErrorClass) {
      return new ErrorClass(stderr, exitCode, stderr);
    }
    return new PdftractError(stderr, exitCode, stderr);
  }

  private async exec(args: string[], timeout?: number): Promise<string> {
    try {
      const result = await spawnPdftract<string>(args, undefined, { timeout });
      return typeof result === 'string' ? result : JSON.stringify(result);
    } catch (error: any) {
      // Map subprocess errors to PdftractError hierarchy
      if (error.exitCode !== undefined) {
        throw this.mapError(error.stderr || error.message, error.exitCode);
      }
      throw error;
    }
  }


  /**
   * Extract structured data from a PDF.
   */
  async extract(source: Source, options?: ExtractOptions): Promise<Document> {
    const args = ['extract', ...(await this.sourceArgs(source)), ...this.optionsArgs(options), '--json', '-'];
    const output = await this.exec(args);
    return JSON.parse(output) as Document;
  }

  /**
   * Extract plain text from a PDF.
   */
  async extractText(source: Source, options?: ExtractOptions): Promise<string> {
    const args = ['extract', ...(await this.sourceArgs(source)), ...this.optionsArgs(options), '--text', '-'];
    const output = await this.exec(args);
    return output;
  }

  /**
   * Extract Markdown-formatted text from a PDF.
   */
  async extractMarkdown(source: Source, options?: ExtractOptions): Promise<string> {
    const args = ['extract', ...(await this.sourceArgs(source)), ...this.optionsArgs(options), '--md', '-'];
    const output = await this.exec(args);
    return output;
  }

  /**
   * Extract pages from a PDF as a stream.
   */
  async *extractStream(source: Source, options?: ExtractOptions): AsyncIterable<Page> {
    const args = ['extract', '--ndjson', ...(await this.sourceArgs(source)), ...this.optionsArgs(options)];
    try {
      for await (const item of spawnPdftractStream<Page>(args)) {
        yield item;
      }
    } catch (error: any) {
      if (error.exitCode !== undefined) {
        throw this.mapError(error.stderr || error.message, error.exitCode);
      }
      throw error;
    }
  }

  /**
   * Extract pages from a PDF as a Node.js Readable stream.
   *
   * Returns a Readable stream that emits Page objects. This is useful for
   * integrating with other Node.js streaming APIs.
   *
   * @example
   * ```ts
   * const stream = await client.extractStreamReadable(source, options);
   * stream.on('data', (page) => console.log(page));
   * stream.on('end', () => console.log('Done'));
   * stream.on('error', (err) => console.error(err));
   * ```
   */
  async extractStreamReadable(source: Source, options?: ExtractOptions & NdjsonReadableOptions): Promise<NdjsonReadable<Page>> {
    const args = ['extract', '--ndjson', ...(await this.sourceArgs(source)), ...this.optionsArgs(options)];
    return createExtractStream(args, {
      timeout: options?.timeout,
      env: options?.env,
      highWaterMark: options?.highWaterMark,
    });
  }

  /**
   * Search for text in a PDF.
   *
   * Note: This feature requires the 'grep' subcommand which is not yet available
   * in the current CLI version (planned for Phase 7.8). This method will throw
   * a descriptive error if called.
   */
  async *search(source: Source, pattern: string, options?: SearchOptions): AsyncIterable<Match> {
    throw new PdftractError(
      'Search functionality is not yet available in this version. The \'grep\' subcommand is planned for Phase 7.8.',
      0,
      'Search functionality requires the grep CLI subcommand which is not yet implemented.'
    );
  }

  /**
   * Search for text in a PDF as a Node.js Readable stream.
   *
   * Returns a Readable stream that emits Match objects. This is useful for
   * integrating with other Node.js streaming APIs.
   *
   * @example
   * ```ts
   * const stream = client.searchReadable(source, 'pattern', options);
   * stream.on('data', (match) => console.log(match));
   * stream.on('end', () => console.log('Done'));
   * stream.on('error', (err) => console.error(err));
   * ```
   */
  async searchReadable(source: Source, pattern: string, options?: SearchOptions & NdjsonReadableOptions): Promise<NdjsonReadable<Match>> {
    const args = ['grep', ...(await this.sourceArgs(source)), pattern, ...this.optionsArgs(options)];
    return createSearchStream(args, {
      timeout: options?.timeout,
      env: options?.env,
      highWaterMark: options?.highWaterMark,
    });
  }

  /**
   * Get metadata from a PDF.
   */
  async getMetadata(source: Source, options?: BaseOptions): Promise<Metadata> {
    // Extract full document and return metadata
    // Note: --metadata-only flag doesn't exist in current CLI
    const doc = await this.extract(source, options as ExtractOptions);
    return doc.metadata;
  }

  /**
   * Compute hash fingerprint of a PDF.
   */
  async hash(source: Source, options?: BaseOptions): Promise<Fingerprint> {
    const args = ['hash', ...(await this.sourceArgs(source)), ...this.optionsArgs(options)];
    const output = await this.exec(args);
    return JSON.parse(output) as Fingerprint;
  }

  /**
   * Classify a PDF document.
   */
  async classify(source: Source): Promise<Classification> {
    const args = ['classify', ...(await this.sourceArgs(source))];
    const output = await this.exec(args);
    return JSON.parse(output) as Classification;
  }

  /**
   * Verify a receipt.
   */
  async verifyReceipt(path: string, receipt: string): Promise<boolean> {
    const output = await this.exec(['verify-receipt', path, receipt]);
    return output.trim() === 'true';
  }

  private async sourceArgs(source: Source): Promise<string[]> {
    return source.toArgs();
  }

  private optionsArgs(options?: ExtractOptions | SearchOptions | BaseOptions): string[] {
    if (!options) {
      return [];
    }

    // Validate options before normalizing
    validateOptions(options);

    // Use the ergonomics layer for normalization
    return normalizeOptions(options);
  }
}

export function path(path: string): PathSource {
  return new PathSource(path);
}

export function url(url: string): URLSource {
  return new URLSource(url);
}

export function bytes(bytes: Uint8Array): BytesSource {
  return new BytesSource(bytes);
}
