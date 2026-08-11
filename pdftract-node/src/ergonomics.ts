/**
 * Ergonomics and type-coercion utilities for @pdftract/sdk
 *
 * This module provides helper functions for normalizing options,
 * coercing types, and handling common patterns in a Node.js-idiomatic way.
 */

import type { ExtractOptions, SearchOptions, BaseOptions } from './codegen/types.js';

/**
 * Normalize options object by converting camelCase to CLI-argument format.
 * This is used internally by the Client class but exposed for advanced use cases.
 *
 * @param options - Options object with camelCase keys
 * @returns Array of CLI argument strings
 */
export function normalizeOptions(
  options: ExtractOptions | SearchOptions | BaseOptions
): string[] {
  const args: string[] = [];

  if (!options) {
    return args;
  }

  // ExtractOptions
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

  // SearchOptions
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

  // Note: timeout option exists in SDK types but not in current CLI
  // It will be added in a future CLI version
  // if ('timeout' in options && options.timeout !== undefined) {
  //   args.push('--timeout', String(options.timeout));
  // }

  return args;
}

/**
 * Type guard to check if a string is a URL.
 *
 * @param str - String to check
 * @returns true if string appears to be a URL
 */
export function isUrl(str: string): boolean {
  return str.startsWith('http://') || str.startsWith('https://');
}

/**
 * Type guard to check if a value is a Buffer.
 *
 * @param value - Value to check
 * @returns true if value is a Buffer
 */
export function isBuffer(value: unknown): value is Buffer {
  return Buffer.isBuffer(value);
}

/**
 * Coerce a source value to a normalized source object.
 * Handles strings, URLs, Buffers, and already-normalized Source objects.
 *
 * @param source - Path (string), URL (string or URL), Buffer, or Source object
 * @returns Normalized Source object
 */
export function coerceSource(source: string | URL | Buffer | any): any {
  // If it's already a Source object with toArgs method, return as-is
  if (source && typeof source.toArgs === 'function') {
    return source;
  }

  // Handle URL objects
  if (source instanceof URL) {
    return source;
  }

  // Handle Buffers
  if (Buffer.isBuffer(source)) {
    return source;
  }

  // Handle strings - distinguish between URLs and paths
  if (typeof source === 'string') {
    return source; // Will be handled by normalizeSource in index.ts
  }

  throw new TypeError(
    `Invalid source type: ${typeof source}. Expected string, URL, Buffer, or Source object.`
  );
}

/**
 * Merge multiple options objects, with later options taking precedence.
 * Useful for combining default options with user-provided options.
 *
 * @param defaults - Default options
 * @param overrides - User-provided options to override defaults
 * @returns Merged options object
 */
export function mergeOptions<T extends ExtractOptions | SearchOptions | BaseOptions>(
  defaults: T,
  overrides: Partial<T>
): T {
  return { ...defaults, ...overrides };
}

/**
 * Create a lazy options object that only evaluates when accessed.
 * Useful for expensive option computations that may not be needed.
 *
 * @param factory - Function that produces options
 * @returns Options object
 */
export function lazyOptions<T extends ExtractOptions | SearchOptions | BaseOptions>(
  factory: () => T
): T {
  return factory();
}

/**
 * Validate options object and throw descriptive errors for invalid values.
 *
 * @param options - Options to validate
 * @throws TypeError with descriptive message for invalid options
 */
export function validateOptions(options: ExtractOptions | SearchOptions | BaseOptions): void {
  if (!options || typeof options !== 'object') {
    throw new TypeError('Options must be an object');
  }

  // Validate numeric options
  if ('ocrThreshold' in options && options.ocrThreshold !== undefined) {
    if (typeof options.ocrThreshold !== 'number' || options.ocrThreshold < 0 || options.ocrThreshold > 100) {
      throw new TypeError('ocrThreshold must be a number between 0 and 100');
    }
  }

  if ('minImageSize' in options && options.minImageSize !== undefined) {
    if (typeof options.minImageSize !== 'number' || options.minImageSize < 0) {
      throw new TypeError('minImageSize must be a non-negative number');
    }
  }

  if ('maxResults' in options && options.maxResults !== undefined) {
    if (typeof options.maxResults !== 'number' || options.maxResults < 0) {
      throw new TypeError('maxResults must be a non-negative number');
    }
  }

  if ('timeout' in options && options.timeout !== undefined) {
    if (typeof options.timeout !== 'number' || options.timeout < 0) {
      throw new TypeError('timeout must be a non-negative number');
    }
  }

  // Validate string options
  if ('ocrLanguage' in options && options.ocrLanguage !== undefined) {
    if (typeof options.ocrLanguage !== 'string' || options.ocrLanguage.length === 0) {
      throw new TypeError('ocrLanguage must be a non-empty string');
    }
  }

  if ('imageFormat' in options && options.imageFormat !== undefined) {
    const validFormats = ['png', 'jpeg', 'webp'];
    if (typeof options.imageFormat !== 'string' || !validFormats.includes(options.imageFormat)) {
      throw new TypeError(`imageFormat must be one of: ${validFormats.join(', ')}`);
    }
  }
}
