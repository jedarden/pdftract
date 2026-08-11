/**
 * stream.ts - Streaming interface for PDF processing
 *
 * Provides NdjsonReadable, a Node.js Readable stream that emits parsed JSON objects
 * from the pdftract CLI's NDJSON output mode.
 */

import { Readable } from 'stream';
import { spawn } from 'child_process';
import { spawnPdftractStream } from './subprocess.js';

/**
 * Options for creating an NdjsonReadable stream
 */
export interface NdjsonReadableOptions {
  /** Timeout in milliseconds (default: 30000) */
  timeout?: number;
  /** Custom environment variables */
  env?: NodeJS.ProcessEnv;
  /** HighWaterMark for backpressure control (default: 16) */
  highWaterMark?: number;
  /** Binary path to use (if not using default pdftract) */
  binaryPath?: string;
}

/**
 * Error thrown when the stream encounters an error during parsing or execution
 */
export class StreamError extends Error {
  constructor(
    message: string,
    public readonly exitCode?: number,
    public readonly stderr?: string
  ) {
    super(message);
    this.name = 'StreamError';
  }
}

/**
 * Generic NDJSON subprocess stream generator.
 * Spawns any command and yields parsed JSON objects from stdout.
 *
 * @param command - Command to run (e.g., 'pdftract', 'echo', 'sh')
 * @param args - Arguments to pass to the command
 * @param options - Spawn options
 * @returns Async generator yielding parsed JSON objects
 */
async function* spawnNdjsonCommand<T = any>(
  command: string,
  args: string[],
  options: NdjsonReadableOptions = {}
): AsyncGenerator<T> {
  const child = spawn(command, args, {
    stdio: ['pipe', 'pipe', 'pipe'],
    env: { ...process.env, ...options.env },
  });

  let buffer = '';
  let stderrBuffer = '';
  let isTimedOut = false;

  // Set up timeout
  if (options.timeout && options.timeout > 0) {
    const timeoutId = setTimeout(() => {
      isTimedOut = true;
      child.kill('SIGTERM');
    }, options.timeout);

    child.on('close', () => clearTimeout(timeoutId));
  }

  child.stderr?.on('data', (chunk) => {
    stderrBuffer += chunk.toString();
  });

  try {
    for await (const chunk of child.stdout!) {
      buffer += chunk.toString();
      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        if (line.trim()) {
          try {
            yield JSON.parse(line) as T;
          } catch (error) {
            throw new Error(`Failed to parse NDJSON line: ${(error as Error).message}`);
          }
        }
      }
    }

    // Handle remaining buffer
    if (buffer.trim()) {
      try {
        yield JSON.parse(buffer) as T;
      } catch (error) {
        throw new Error(`Failed to parse NDJSON line: ${(error as Error).message}`);
      }
    }

    // Wait for process to complete
    const exitCode = await new Promise<number>((resolve) => {
      child.on('close', resolve);
    });

    if (exitCode !== 0 && !isTimedOut) {
      const error: any = new Error(stderrBuffer || `Command exited with code ${exitCode}`);
      error.exitCode = exitCode;
      error.stderr = stderrBuffer;
      throw error;
    }
  } catch (error) {
    child.kill();
    throw error;
  }
}

/**
 * A Readable stream that emits parsed JSON objects from NDJSON output.
 *
 * This stream can work with any command that produces NDJSON output.
 * By default, it uses the pdftract binary, but can be configured to use
 * any command.
 *
 * @example
 * ```ts
 * const stream = new NdjsonReadable(['extract', '--ndjson', 'doc.pdf'], Page);
 * for await (const page of stream) {
 *   console.log(page);
 * }
 * ```
 *
 * @example
 * ```ts
 * const stream = new NdjsonReadable(['grep', 'doc.pdf', 'pattern'], Match);
 * stream.on('data', (match: Match) => console.log(match));
 * stream.on('error', (err) => console.error(err));
 * ```
 */
export class NdjsonReadable<T = any> extends Readable {
  private generator: AsyncGenerator<T>;
  private isReading: boolean = false;
  private isEnded: boolean = false;

  /**
   * Create a new NdjsonReadable stream
   *
   * @param args - Command and arguments (first element is the command)
   * @param _type - Type parameter for type inference (not used at runtime)
   * @param options - Stream options (timeout, env, highWaterMark, binaryPath)
   */
  constructor(
    args: string[],
    private _type: new (...args: any[]) => T,
    options: NdjsonReadableOptions = {}
  ) {
    super({
      objectMode: true, // Emit parsed objects, not buffers
      highWaterMark: options.highWaterMark ?? 16,
    });

    // Use custom binary path if provided, otherwise use first arg as command
    const command = options.binaryPath || args[0];
    // If binaryPath is provided and matches args[0], skip the first arg
    // Otherwise, if binaryPath is provided, use all args
    // If no binaryPath, skip the first arg (it's the command)
    let commandArgs: string[];
    if (options.binaryPath) {
      commandArgs = args[0] === options.binaryPath ? args.slice(1) : args;
    } else {
      commandArgs = args.slice(1);
    }

    this.generator = spawnNdjsonCommand<T>(command, commandArgs, options);
  }

  /**
   * Internal read method called by Node.js when the stream wants data.
   * This implements backpressure by only reading from the generator when
   * the stream's buffer is not full.
   */
  async _read(): Promise<void> {
    // Prevent concurrent reads
    if (this.isReading || this.isEnded) {
      return;
    }

    this.isReading = true;

    try {
      const { done, value } = await this.generator.next();

      if (done) {
        this.isEnded = true;
        this.push(null); // Signal EOF
      } else {
        // Push the parsed JSON object to the stream
        // If false is returned, the buffer is full and we should wait
        const continueReading = this.push(value);

        if (!continueReading) {
          // Backpressure: buffer is full, wait for next _read call
          this.isReading = false;
        } else {
          // Continue reading immediately
          // Use setImmediate to avoid stack overflow on fast producers
          setImmediate(() => {
            this.isReading = false;
            this._read().catch((err) => this._onError(err));
          });
        }
      }
    } catch (error: any) {
      this._onError(error);
    } finally {
      this.isReading = false;
    }
  }

  /**
   * Handle errors from the generator or subprocess
   */
  private _onError(error: any): void {
    if (this.destroyed) {
      return;
    }

    let streamError: Error;

    if (error.exitCode !== undefined) {
      // Error from subprocess
      streamError = new StreamError(
        error.message || 'Subprocess failed',
        error.exitCode,
        error.stderr
      );
    } else {
      // Parsing or other error
      streamError = new StreamError(error.message || 'Stream processing failed');
    }

    this.destroy(streamError);
  }

  /**
   * Cleanup when the stream is destroyed
   */
  async _destroy(error: Error | null, callback: (error?: Error | null) => void): Promise<void> {
    try {
      // Return from the generator to cleanup the subprocess
      await this.generator.return(undefined);
    } catch (cleanupError) {
      // Ignore cleanup errors
    }

    callback(error);
  }
}

/**
 * Create a streaming extract operation that emits Page objects
 *
 * @param args - Command arguments for the extract operation (excluding 'pdftract')
 * @param options - Stream options
 * @returns Readable stream emitting Page objects
 */
export function createExtractStream(
  args: string[],
  options?: NdjsonReadableOptions
): NdjsonReadable {
  // When binaryPath is provided, use it as the command; otherwise use 'pdftract'
  // The args array should NOT include the binary name - NdjsonReadable will add it
  const binary = options?.binaryPath || 'pdftract';
  return new NdjsonReadable([binary, ...args], Object as any, options);
}

/**
 * Create a streaming search operation that emits Match objects
 *
 * @param args - Command arguments for the search operation (excluding 'pdftract')
 * @param options - Stream options
 * @returns Readable stream emitting Match objects
 */
export function createSearchStream(
  args: string[],
  options?: NdjsonReadableOptions
): NdjsonReadable {
  // When binaryPath is provided, use it as the command; otherwise use 'pdftract'
  // The args array should NOT include the binary name - NdjsonReadable will add it
  const binary = options?.binaryPath || 'pdftract';
  return new NdjsonReadable([binary, ...args], Object as any, options);
}
