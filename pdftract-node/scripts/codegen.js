#!/usr/bin/env node

/**
 * Code generator for pdftract Node.js SDK
 * Generates methods.ts and errors.ts from templates
 */

import { writeFile, mkdir } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Error class definitions
const ERROR_CLASSES = [
  {
    name: 'PdftractError',
    base: 'Error',
    exitCode: null,
    comment: 'Base error for all pdftract errors'
  },
  {
    name: 'CorruptPdfError',
    base: 'PdftractError',
    exitCode: 2,
    comment: 'Corrupt PDF'
  },
  {
    name: 'EncryptionError',
    base: 'PdftractError',
    exitCode: 3,
    comment: 'Encrypted / password missing/wrong'
  },
  {
    name: 'SourceUnreachableError',
    base: 'PdftractError',
    exitCode: 4,
    comment: 'Source unreadable'
  },
  {
    name: 'RemoteFetchInterruptedError',
    base: 'PdftractError',
    exitCode: 5,
    comment: 'Network interrupted'
  },
  {
    name: 'TlsError',
    base: 'PdftractError',
    exitCode: 6,
    comment: 'TLS / cert failure'
  },
  {
    name: 'ReceiptVerifyError',
    base: 'PdftractError',
    exitCode: 10,
    comment: 'Receipt verify failed'
  },
  {
    name: 'ValidationError',
    base: 'PdftractError',
    exitCode: 1,
    comment: 'Input validation failed'
  }
];

// Method definitions
const METHODS = [
  {
    name: 'extract',
    async: true,
    source: true,
    options: 'ExtractOptions',
    returnType: 'Document',
    subcommand: 'extract',
    comment: 'Extract structured data from a PDF.'
  },
  {
    name: 'extractText',
    async: true,
    source: true,
    options: 'ExtractOptions',
    returnType: 'string',
    subcommand: 'extract',
    extraArgs: ['--text'],
    comment: 'Extract plain text from a PDF.'
  },
  {
    name: 'extractMarkdown',
    async: true,
    source: true,
    options: 'ExtractOptions',
    returnType: 'string',
    subcommand: 'extract',
    extraArgs: ['--md'],
    comment: 'Extract Markdown-formatted text from a PDF.'
  },
  {
    name: 'extractStream',
    generator: true,
    source: true,
    options: 'ExtractOptions',
    yieldType: 'Page',
    subcommand: 'extract',
    extraArgs: ['--ndjson'],
    comment: 'Extract pages from a PDF as a stream.'
  },
  {
    name: 'search',
    generator: true,
    source: true,
    options: 'SearchOptions',
    extraParams: ['pattern: string'],
    yieldType: 'Match',
    subcommand: 'grep',
    comment: 'Search for text in a PDF.'
  },
  {
    name: 'getMetadata',
    async: true,
    source: true,
    options: 'BaseOptions',
    returnType: 'Metadata',
    subcommand: 'extract',
    extraArgs: ['--metadata-only'],
    comment: 'Get metadata from a PDF.'
  },
  {
    name: 'hash',
    async: true,
    source: true,
    options: 'BaseOptions',
    returnType: 'Fingerprint',
    subcommand: 'hash',
    comment: 'Compute hash fingerprint of a PDF.'
  },
  {
    name: 'classify',
    async: true,
    source: true,
    options: null,
    returnType: 'Classification',
    subcommand: 'classify',
    comment: 'Classify a PDF document.'
  },
  {
    name: 'verifyReceipt',
    async: true,
    source: false,
    options: null,
    extraParams: ['path: string', 'receipt: string'],
    returnType: 'boolean',
    subcommand: 'verify-receipt',
    customImpl: true,
    comment: 'Verify a receipt.'
  }
];

function generateErrors() {
  let output = `/**
 * This file is auto-generated. Do not edit manually.
 */

`;

  ERROR_CLASSES.forEach(error => {
    output += '\n';
    if (error.comment) {
      output += `/**\n * ${error.comment}\n */\n`;
    }

    if (error.base === 'Error') {
      output += `export class ${error.name} extends Error {
  constructor(
    message: string,
    public readonly exitCode: number,
    public readonly stderr: string
  ) {
    super(message);
    this.name = '${error.name}';
  }
}
`;
    } else {
      output += `export class ${error.name} extends ${error.base} {
  constructor(message: string, exitCode: number, stderr: string) {
    super(message, exitCode, stderr);
    this.name = '${error.name}';
  }
}
`;
    }
  });

  return output;
}

function generateMethods() {
  let output = `/**
 * This file is auto-generated. Do not edit manually.
 */

import { spawnPdftract, spawnPdftractStream } from '../subprocess.js';
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

`;

  // Generate each method
  METHODS.forEach(method => {
    output += '\n';
    if (method.comment) {
      output += `  /**\n   * ${method.comment}\n   */\n`;
    }

    let params = [];
    if (method.source) {
      params.push('source: Source');
    }
    if (method.extraParams) {
      params.push(...method.extraParams);
    }
    if (method.options) {
      params.push(`options?: ${method.options}`);
    }

    if (method.async) {
      output += `  async ${method.name}(${params.join(', ')}): Promise<${method.returnType}> {\n`;

      let args = [`'${method.subcommand}'`];
      if (method.source) {
        args.push('...(await this.sourceArgs(source))');
      }
      if (method.extraParams) {
        if (method.name === 'search') {
          args.push('pattern');
        } else if (method.name === 'verifyReceipt') {
          args.push('path', 'receipt');
        }
      }
      if (method.options) {
        args.push('...this.optionsArgs(options)');
      }
      if (method.extraArgs) {
        args.push(...method.extraArgs.map(a => `'${a}'`));
      }

      if (method.customImpl) {
        output += `    const output = await this.exec([${args.join(', ')}]);\n`;
        output += `    return output.trim() === 'true';\n`;
      } else if (method.returnType === 'string') {
        output += `    const args = [${args.join(', ')}];\n`;
        output += `    const output = await this.exec(args);\n`;
        output += `    return output;\n`;
      } else {
        output += `    const args = [${args.join(', ')}];\n`;
        output += `    const output = await this.exec(args);\n`;
        output += `    return JSON.parse(output) as ${method.returnType};\n`;
      }

      output += `  }\n`;
    } else if (method.generator) {
      output += `  async *${method.name}(${params.join(', ')}): AsyncIterable<${method.yieldType}> {\n`;

      let args = [`'${method.subcommand}'`];
      if (method.extraArgs) {
        args.push(...method.extraArgs.map(a => `'${a}'`));
      }
      if (method.source) {
        args.push('...(await this.sourceArgs(source))');
      }
      if (method.options) {
        args.push('...this.optionsArgs(options)');
      }
      if (method.name === 'search') {
        args.splice(2, 0, 'pattern');
      }

      output += `    const args = [${args.join(', ')}];\n`;
      output += `    try {\n`;
      output += `      for await (const item of spawnPdftractStream<${method.yieldType}>(args)) {\n`;
      output += `        yield item;\n`;
      output += `      }\n`;
      output += `    } catch (error: any) {\n`;
      output += `      if (error.exitCode !== undefined) {\n`;
      output += `        throw this.mapError(error.stderr || error.message, error.exitCode);\n`;
      output += `      }\n`;
      output += `      throw error;\n`;
      output += `    }\n`;
      output += `  }\n`;
    }
  });

  // Helper methods
  output += `
  private async sourceArgs(source: Source): Promise<string[]> {
    return source.toArgs();
  }

  private optionsArgs(options?: ExtractOptions | SearchOptions | BaseOptions): string[] {
    const args: string[] = [];
    if (!options) return args;

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
`;

  return output;
}

async function main() {
  console.log('Generating pdftract Node.js SDK code...');

  // Generate errors.ts
  const errorsPath = join(__dirname, '..', 'src', 'codegen', 'errors.ts');
  await mkdir(dirname(errorsPath), { recursive: true });
  await writeFile(errorsPath, generateErrors(), 'utf-8');
  console.log('✓ Generated src/codegen/errors.ts');

  // Generate methods.ts
  const methodsPath = join(__dirname, '..', 'src', 'codegen', 'methods.ts');
  await writeFile(methodsPath, generateMethods(), 'utf-8');
  console.log('✓ Generated src/codegen/methods.ts');

  console.log('\nCode generation complete!');
}

main().catch(console.error);
