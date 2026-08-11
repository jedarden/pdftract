/**
 * Standalone functions for @pdftract/sdk
 *
 * These functions provide a convenient API using a default Client instance.
 * For advanced use cases, create a custom Client instance with specific binary paths.
 */

import { Client, path as pathFn, url as urlFn, bytes as bytesFn } from './codegen/methods.js';
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
  HashOptions,
  Receipt
} from './codegen/types.js';
import { NdjsonReadable } from './stream.js';
import type { NdjsonReadableOptions } from './stream.js';

// Default client instance
const defaultClient = new Client();

/**
 * Convert a source value to a Source object.
 *
 * @param source - PDF file path, URL, Buffer, or Source object
 * @returns Source object
 */
function toSource(source: string | URL | Buffer | Source): Source {
  // If it's already a Source object (has toArgs method), return as-is
  if (source && typeof (source as any).toArgs === 'function') {
    return source as Source;
  }
  if (source instanceof URL) {
    return urlFn(source.toString());
  }
  if (Buffer.isBuffer(source)) {
    return bytesFn(source);
  }
  if (typeof source === 'string') {
    return pathFn(source);
  }
  throw new TypeError(
    `Invalid source type: ${typeof source}. Expected string, URL, or Buffer.`
  );
}

/**
 * Extract structured data from a PDF.
 *
 * @param source - PDF file path, URL, or Buffer
 * @param options - Extraction options
 * @returns Promise resolving to Document object
 */
export async function extract(
  source: string | URL | Buffer,
  options?: ExtractOptions
): Promise<Document> {
  return defaultClient.extract(toSource(source), options);
}

/**
 * Extract plain text from a PDF.
 *
 * @param source - PDF file path, URL, or Buffer
 * @param options - Extraction options
 * @returns Promise resolving to extracted text string
 */
export async function extractText(
  source: string | URL | Buffer,
  options?: ExtractOptions
): Promise<string> {
  return defaultClient.extractText(toSource(source), options);
}

/**
 * Extract Markdown-formatted text from a PDF.
 *
 * @param source - PDF file path, URL, or Buffer
 * @param options - Extraction options
 * @returns Promise resolving to Markdown text string
 */
export async function extractMarkdown(
  source: string | URL | Buffer,
  options?: ExtractOptions
): Promise<string> {
  return defaultClient.extractMarkdown(toSource(source), options);
}

/**
 * Extract pages from a PDF as an async iterable stream.
 *
 * @param source - PDF file path, URL, or Buffer
 * @param options - Extraction options
 * @returns AsyncIterable yielding Page objects
 */
export async function* extractStream(
  source: string | URL | Buffer,
  options?: ExtractOptions
): AsyncIterable<Page> {
  yield* defaultClient.extractStream(toSource(source), options);
}

/**
 * Extract pages from a PDF as a Node.js Readable stream.
 *
 * @param source - PDF file path, URL, or Buffer
 * @param options - Extraction options
 * @returns Promise resolving to Readable stream emitting Page objects
 */
export async function extractReadable(
  source: string | URL | Buffer,
  options?: ExtractOptions & NdjsonReadableOptions
): Promise<NdjsonReadable<Page>> {
  return defaultClient.extractStreamReadable(toSource(source), options);
}

/**
 * Search for text patterns in a PDF as an async iterable stream.
 *
 * @param source - PDF file path, URL, or Buffer
 * @param pattern - Text pattern or RegExp to search for
 * @param options - Search options
 * @returns AsyncIterable yielding Match objects
 */
export async function* search(
  source: string | URL | Buffer,
  pattern: string | RegExp,
  options?: SearchOptions
): AsyncIterable<Match> {
  yield* defaultClient.search(toSource(source), pattern.toString(), options);
}

/**
 * Search for text patterns in a PDF as a Node.js Readable stream.
 *
 * @param source - PDF file path, URL, or Buffer
 * @param pattern - Text pattern or RegExp to search for
 * @param options - Search options
 * @returns Promise resolving to Readable stream emitting Match objects
 */
export async function searchReadable(
  source: string | URL | Buffer,
  pattern: string | RegExp,
  options?: SearchOptions & NdjsonReadableOptions
): Promise<NdjsonReadable<Match>> {
  return defaultClient.searchReadable(toSource(source), pattern.toString(), options);
}

/**
 * Get metadata from a PDF.
 *
 * @param source - PDF file path, URL, or Buffer
 * @param options - Extraction options
 * @returns Promise resolving to Metadata object
 */
export async function getMetadata(
  source: string | URL | Buffer,
  options?: ExtractOptions
): Promise<Metadata> {
  return defaultClient.getMetadata(toSource(source), options as any);
}

/**
 * Compute hash fingerprint of a PDF.
 *
 * @param source - PDF file path, URL, or Buffer
 * @param options - Hash options
 * @returns Promise resolving to Fingerprint object
 */
export async function hash(
  source: string | URL | Buffer,
  options?: HashOptions
): Promise<Fingerprint> {
  return defaultClient.hash(toSource(source), options);
}

/**
 * Classify a PDF document.
 *
 * @param source - PDF file path, URL, or Buffer
 * @returns Promise resolving to Classification object
 */
export async function classify(
  source: string | URL | Buffer
): Promise<Classification> {
  return defaultClient.classify(toSource(source));
}

/**
 * Verify a receipt's authenticity.
 *
 * @param path - Path to the PDF file
 * @param receipt - Receipt object or JSON string
 * @returns Promise resolving to boolean indicating validity
 */
export async function verifyReceipt(
  path: string,
  receipt: Receipt | string
): Promise<boolean> {
  const receiptString = typeof receipt === 'string' ? receipt : JSON.stringify(receipt);
  return defaultClient.verifyReceipt(path, receiptString);
}
