/**
 * This file is auto-generated. Do not edit manually.
 */


/**
 * Base error for all pdftract errors
 */
export class PdftractError extends Error {
  constructor(
    message: string,
    public readonly exitCode: number,
    public readonly stderr: string
  ) {
    super(message);
    this.name = 'PdftractError';
  }
}

/**
 * Corrupt PDF
 */
export class CorruptPdfError extends PdftractError {
  constructor(message: string, exitCode: number, stderr: string) {
    super(message, exitCode, stderr);
    this.name = 'CorruptPdfError';
  }
}

/**
 * Encrypted / password missing/wrong
 */
export class EncryptionError extends PdftractError {
  constructor(message: string, exitCode: number, stderr: string) {
    super(message, exitCode, stderr);
    this.name = 'EncryptionError';
  }
}

/**
 * Source unreadable
 */
export class SourceUnreachableError extends PdftractError {
  constructor(message: string, exitCode: number, stderr: string) {
    super(message, exitCode, stderr);
    this.name = 'SourceUnreachableError';
  }
}

/**
 * Network interrupted
 */
export class RemoteFetchInterruptedError extends PdftractError {
  constructor(message: string, exitCode: number, stderr: string) {
    super(message, exitCode, stderr);
    this.name = 'RemoteFetchInterruptedError';
  }
}

/**
 * TLS / cert failure
 */
export class TlsError extends PdftractError {
  constructor(message: string, exitCode: number, stderr: string) {
    super(message, exitCode, stderr);
    this.name = 'TlsError';
  }
}

/**
 * Receipt verify failed
 */
export class ReceiptVerifyError extends PdftractError {
  constructor(message: string, exitCode: number, stderr: string) {
    super(message, exitCode, stderr);
    this.name = 'ReceiptVerifyError';
  }
}

/**
 * Input validation failed
 */
export class ValidationError extends PdftractError {
  constructor(message: string, exitCode: number, stderr: string) {
    super(message, exitCode, stderr);
    this.name = 'ValidationError';
  }
}
