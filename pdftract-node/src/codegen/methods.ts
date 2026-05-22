/**
 * This file is auto-generated. Do not edit manually.
 */

import { spawn } from 'child_process';
import type {
  Source,
  PathSource,
  URLSource,
  BytesSource,
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
  ReceiptVerifyError
} from './errors.js';

/**
 * Maps exit codes to error classes.
 */
const ERROR_MAP: Record<number, typeof PdftractError> = {
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

  private async exec(args: string[]): Promise<string> {
    const { spawn } = await import('child_process');

    return new Promise((resolve, reject) => {
      const child = spawn(this.binaryPath, args);
      let stdout = '';
      let stderr = '';

      child.stdout?.on('data', (chunk) => {
        stdout += chunk.toString();
      });

      child.stderr?.on('data', (chunk) => {
        stderr += chunk.toString();
      });

      child.on('close', (code) => {
        if (code === 0) {
          resolve(stdout);
        } else {
          reject(this.mapError(stderr, code || 1));
        }
      });

      child.on('error', (err) => {
        reject(new PdftractError(err.message, 1, stderr));
      });
    });
  }

  /**
   * Extract structured data from a PDF.
   */
  async extract(
    source: Source,
    options?: ExtractOptions
  ): Promise<Document> {
    const args = ['extract', ...(await this.sourceArgs(source))];

    if (options) {
      args.push(...this.optionsArgs(options));
    }

    const output = await this.exec(args);
    return JSON.parse(output) as Document;
  }

  /**
   * Extract plain text from a PDF.
   */
  async extractText(
    source: Source,
    options?: ExtractOptions
  ): Promise<string> {
    const args = ['extract', ...(await this.sourceArgs(source))];

    if (options) {
      args.push(...this.optionsArgs(options));
    }

    args.push('--text');

    const output = await this.exec(args);
    return output;
  }

  /**
   * Extract Markdown-formatted text from a PDF.
   */
  async extractMarkdown(
    source: Source,
    options?: ExtractOptions
  ): Promise<string> {
    const args = ['extract', ...(await this.sourceArgs(source))];

    if (options) {
      args.push(...this.optionsArgs(options));
    }

    args.push('--md');

    const output = await this.exec(args);
    return output;
  }

  /**
   * Extract pages from a PDF as a stream.
   */
  async *extractStream(
    source: Source,
    options?: ExtractOptions
  ): AsyncIterable<Page> {
    const args = ['extract', '--ndjson', ...(await this.sourceArgs(source))];
    if (options) {
      args.push(...this.optionsArgs(options));
    }

    const child = spawn(this.binaryPath, args);
    const errorChunks: Buffer[] = [];

    child.stderr?.on('data', (chunk) => errorChunks.push(chunk));

    try {
      let buffer = '';
      for await (const chunk of child.stdout!) {
        buffer += chunk.toString();
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (line.trim()) {
            yield JSON.parse(line) as Page;
          }
        }
      }

      if (buffer.trim()) {
        yield JSON.parse(buffer) as Page;
      }

      const exitCode = await new Promise<number>((resolve) => {
        child.on('close', resolve);
      });

      if (exitCode !== 0) {
        const stderr = Buffer.concat(errorChunks).toString();
        throw this.mapError(stderr, exitCode);
      }
    } catch (error) {
      child.kill();
      throw error;
    }
  }

  /**
   * Search for text in a PDF.
   */
  async *search(
    source: Source,
    pattern: string,
    options?: SearchOptions
  ): AsyncIterable<Match> {
    const args = ['grep', pattern, ...(await this.sourceArgs(source))];
    if (options) {
      args.push(...this.optionsArgs(options));
    }

    const child = spawn(this.binaryPath, args);
    const errorChunks: Buffer[] = [];

    child.stderr?.on('data', (chunk) => errorChunks.push(chunk));

    try {
      let buffer = '';
      for await (const chunk of child.stdout!) {
        buffer += chunk.toString();
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (line.trim()) {
            yield JSON.parse(line) as Match;
          }
        }
      }

      if (buffer.trim()) {
        yield JSON.parse(buffer) as Match;
      }

      const exitCode = await new Promise<number>((resolve) => {
        child.on('close', resolve);
      });

      if (exitCode !== 0) {
        const stderr = Buffer.concat(errorChunks).toString();
        throw this.mapError(stderr, exitCode);
      }
    } catch (error) {
      child.kill();
      throw error;
    }
  }

  /**
   * Get metadata from a PDF.
   */
  async getMetadata(
    source: Source,
    options?: BaseOptions
  ): Promise<Metadata> {
    const args = ['extract', '--metadata-only', ...(await this.sourceArgs(source))];

    if (options) {
      args.push(...this.optionsArgs(options));
    }

    const output = await this.exec(args);
    return JSON.parse(output) as Metadata;
  }

  /**
   * Compute hash fingerprint of a PDF.
   */
  async hash(
    source: Source,
    options?: BaseOptions
  ): Promise<Fingerprint> {
    const args = ['hash', ...(await this.sourceArgs(source))];

    if (options) {
      args.push(...this.optionsArgs(options));
    }

    const output = await this.exec(args);
    return JSON.parse(output) as Fingerprint;
  }

  /**
   * Classify a PDF document.
   */
  async classify(
    source: Source
  ): Promise<Classification> {
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

  private optionsArgs(options: ExtractOptions | SearchOptions | BaseOptions): string[] {
    const args: string[] = [];

    if ('ocrLanguage' in options && options.ocrLanguage) {
      args.push('--ocr-language', options.ocrLanguage);
    }
    if ('ocrThreshold' in options && options.ocrThreshold !== undefined) {
      args.push('--ocr-threshold', String(options.ocrThreshold));
    }
    if ('preserveLayout' in options && options.preserveLayout) {
      args.push('--preserve-layout');
    }
    if ('extractImages' in options && options.extractImages) {
      args.push('--extract-images');
    }
    if ('imageFormat' in options && options.imageFormat) {
      args.push('--image-format', options.imageFormat);
    }
    if ('minImageSize' in options && options.minImageSize !== undefined) {
      args.push('--min-image-size', String(options.minImageSize));
    }
    if ('password' in options && options.password) {
      args.push('--password', options.password);
    }
    if ('caseInsensitive' in options && options.caseInsensitive) {
      args.push('--case-insensitive');
    }
    if ('regex' in options && options.regex) {
      args.push('--regex');
    }
    if ('wholeWord' in options && options.wholeWord) {
      args.push('--whole-word');
    }
    if ('maxResults' in options && options.maxResults !== undefined) {
      args.push('--max-results', String(options.maxResults));
    }
    if ('timeout' in options && options.timeout !== undefined) {
      args.push('--timeout', String(options.timeout));
    }

    return args;
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
