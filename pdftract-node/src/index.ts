/**
 * pdftract Node.js SDK
 * Auto-generated - do not edit manually
 */

export { Client, path, url, bytes } from './codegen/methods.js';

// Export subprocess module for advanced use cases
export {
  spawnPdftract,
  spawnPdftractStream,
  resolveBinaryPath,
  BinaryNotFoundError,
  SpawnError,
} from './subprocess.js';

// Export streaming utilities
export {
  NdjsonReadable,
  StreamError,
  createExtractStream,
  createSearchStream,
} from './stream.js';

export type { SpawnOptions, PdftractErrorResponse } from './subprocess.js';
export type {
  Source,
  PathSource,
  URLSource,
  BytesSource,
  Document,
  Page,
  Span,
  Block,
  Match,
  Fingerprint,
  Classification,
  Metadata,
  ExtractOptions,
  SearchOptions,
  BaseOptions,
  HashOptions,
  Receipt
} from './codegen/types.js';
export type { NdjsonReadableOptions } from './stream.js';

export { PdftractError } from './codegen/errors.js';
export { CorruptPdfError } from './codegen/errors.js';
export { EncryptionError } from './codegen/errors.js';
export { SourceUnreachableError } from './codegen/errors.js';
export { RemoteFetchInterruptedError } from './codegen/errors.js';
export { TlsError } from './codegen/errors.js';
export { ReceiptVerifyError } from './codegen/errors.js';
export { ValidationError } from './codegen/errors.js';

// Export ergonomics utilities for advanced use cases
export {
  normalizeOptions,
  coerceSource,
  validateOptions,
  mergeOptions,
  lazyOptions,
  isUrl,
  isBuffer,
} from './ergonomics.js';
