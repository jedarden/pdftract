/**
 * Unit tests for @pdftract/sdk
 */

import { describe, it, expect } from 'vitest';
import {
  Client,
  path,
  url,
  bytes,
  PdftractError,
  CorruptPdfError,
  EncryptionError,
  SourceUnreachableError,
  RemoteFetchInterruptedError,
  TlsError,
  ReceiptVerifyError
} from '../src/index.js';

describe('Client construction', () => {
  it('should create a client with default binary path', () => {
    const client = new Client();
    expect(client).toBeDefined();
  });

  it('should create a client with custom binary path', () => {
    const client = new Client('/custom/path/to/pdftract');
    expect(client).toBeDefined();
  });
});

describe('Source helpers', () => {
  it('should create a PathSource', () => {
    const src = path('/path/to/file.pdf');
    expect(src).toBeDefined();
  });

  it('should create a URLSource', () => {
    const src = url('https://example.com/file.pdf');
    expect(src).toBeDefined();
  });

  it('should create a BytesSource', () => {
    const buffer = Buffer.from('test');
    const src = bytes(buffer);
    expect(src).toBeDefined();
  });
});

describe('Error classes', () => {
  it('should create PdftractError with correct properties', () => {
    const error = new PdftractError('test error', 1, 'stderr output');
    expect(error.message).toBe('test error');
    expect(error.exitCode).toBe(1);
    expect(error.stderr).toBe('stderr output');
    expect(error.name).toBe('PdftractError');
  });

  it('should create CorruptPdfError', () => {
    const error = new CorruptPdfError('corrupt pdf', 2, 'stderr');
    expect(error.name).toBe('CorruptPdfError');
    expect(error.exitCode).toBe(2);
  });

  it('should create EncryptionError', () => {
    const error = new EncryptionError('encrypted pdf', 3, 'stderr');
    expect(error.name).toBe('EncryptionError');
    expect(error.exitCode).toBe(3);
  });

  it('should create SourceUnreachableError', () => {
    const error = new SourceUnreachableError('source unreachable', 4, 'stderr');
    expect(error.name).toBe('SourceUnreachableError');
    expect(error.exitCode).toBe(4);
  });

  it('should create RemoteFetchInterruptedError', () => {
    const error = new RemoteFetchInterruptedError('network error', 5, 'stderr');
    expect(error.name).toBe('RemoteFetchInterruptedError');
    expect(error.exitCode).toBe(5);
  });

  it('should create TlsError', () => {
    const error = new TlsError('tls error', 6, 'stderr');
    expect(error.name).toBe('TlsError');
    expect(error.exitCode).toBe(6);
  });

  it('should create ReceiptVerifyError', () => {
    const error = new ReceiptVerifyError('receipt invalid', 10, 'stderr');
    expect(error.name).toBe('ReceiptVerifyError');
    expect(error.exitCode).toBe(10);
  });

  it('should maintain inheritance chain', () => {
    const corruptError = new CorruptPdfError('test', 2, 'stderr');
    expect(corruptError instanceof PdftractError).toBe(true);
    expect(corruptError instanceof Error).toBe(true);
  });
});

describe('Source argument conversion', () => {
  it('PathSource should return path args', () => {
    const src = path('/path/to/file.pdf');
    const args = src.toArgs();
    expect(args).toEqual(['/path/to/file.pdf']);
  });

  it('URLSource should return URL args', () => {
    const src = url('https://example.com/file.pdf');
    const args = src.toArgs();
    expect(args).toEqual(['https://example.com/file.pdf']);
  });

  it('BytesSource should write temp file and return path', async () => {
    const buffer = Buffer.from('test pdf content');
    const src = bytes(buffer);
    const args = await src.toArgs();
    expect(args).toHaveLength(1);
    expect(args[0]).toMatch(/\.pdf$/);
  });
});
