/**
 * subprocess.ts - Core subprocess spawning and JSON parsing for pdftract SDK
 */

import { spawn, exec } from 'child_process';
import { promisify } from 'util';
import { access, constants } from 'fs/promises';
import { resolve } from 'path';

const execPromise = promisify(exec);

/**
 * Options for spawning the pdftract binary
 */
export interface SpawnOptions {
  /** Timeout in milliseconds (default: 30000) */
  timeout?: number;
  /** Input data to write to stdin as JSON */
  input?: object;
  /** Custom environment variables */
  env?: NodeJS.ProcessEnv;
}

/**
 * Error thrown when the pdftract binary cannot be found
 */
export class BinaryNotFoundError extends Error {
  constructor(binaryPath: string) {
    super(`pdftract binary not found at: ${binaryPath}. Ensure pdftract is installed and in PATH.`);
    this.name = 'BinaryNotFoundError';
  }
}

/**
 * Error thrown when spawning fails for reasons other than "not found"
 */
export class SpawnError extends Error {
  constructor(message: string, public readonly originalError: Error) {
    super(message);
    this.name = 'SpawnError';
  }
}

/**
 * Error response from pdftract (parsed from stderr JSON)
 */
export interface PdftractErrorResponse {
  error: string;
  message?: string;
  details?: string;
}

/**
 * Find the pdftract binary in PATH or at a specific path
 *
 * @param customPath - Optional custom path to the binary
 * @returns Resolved absolute path to the binary
 * @throws BinaryNotFoundError if binary cannot be found
 */
export async function resolveBinaryPath(customPath?: string): Promise<string> {
  // If custom path is provided, check it directly
  if (customPath) {
    const resolvedPath = resolve(customPath);
    try {
      await access(resolvedPath, constants.X_OK);
      return resolvedPath;
    } catch {
      throw new BinaryNotFoundError(resolvedPath);
    }
  }

  // Try to find in PATH using 'which' command
  try {
    const { stdout } = await execPromise('which pdftract');
    if (stdout.trim()) {
      return stdout.trim();
    }
  } catch {
    // 'which' failed, try manual PATH search
  }

  // Manual PATH search as fallback
  const pathDirs = (process.env.PATH || '').split(process.platform === 'win32' ? ';' : ':');

  for (const dir of pathDirs) {
    const possiblePath = resolve(dir, process.platform === 'win32' ? 'pdftract.exe' : 'pdftract');
    try {
      await access(possiblePath, constants.X_OK);
      return possiblePath;
    } catch {
      // Continue searching
    }
  }

  throw new BinaryNotFoundError('pdftract (not found in PATH)');
}

/**
 * Spawn the pdftract binary with JSON input/output handling
 *
 * @param args - Command-line arguments to pass to pdftract
 * @param input - Optional JSON object to write to stdin
 * @param options - Spawn options (timeout, env, etc.)
 * @returns Parsed JSON response from stdout
 * @throws PdftractError on non-zero exit with parsed stderr
 * @throws BinaryNotFoundError if binary cannot be found
 * @throws SpawnError if spawning fails
 */
export async function spawnPdftract<T = any>(
  args: string[],
  input?: object,
  options: SpawnOptions = {}
): Promise<T> {
  const timeout = options.timeout ?? 30000;

  // Resolve binary path
  const binaryPath = await resolveBinaryPath();

  return new Promise<T>((resolve, reject) => {
    // Spawn the process with piped stdin/stdout/stderr
    const child = spawn(binaryPath, args, {
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, ...options.env },
    });

    let stdoutBuffer = '';
    let stderrBuffer = '';
    let isResolved = false;
    let timeoutId: NodeJS.Timeout | null = null;

    // Set up timeout
    if (timeout > 0) {
      timeoutId = setTimeout(() => {
        if (!isResolved) {
          isResolved = true;
          child.kill('SIGTERM');
          reject(new Error(`pdftract timed out after ${timeout}ms`));
        }
      }, timeout);
    }

    // Write JSON input to stdin if provided
    if (input) {
      try {
        const jsonData = JSON.stringify(input);
        child.stdin?.write(jsonData);
        child.stdin?.end();
      } catch (error) {
        cleanup();
        isResolved = true;
        reject(new SpawnError('Failed to write to stdin', error as Error));
        return;
      }
    } else {
      // Close stdin if no input
      child.stdin?.end();
    }

    // Collect stdout
    child.stdout?.on('data', (chunk) => {
      stdoutBuffer += chunk.toString();
    });

    // Collect stderr
    child.stderr?.on('data', (chunk) => {
      stderrBuffer += chunk.toString();
    });

    // Handle spawn errors (binary not found, permission denied, etc.)
    child.on('error', (error) => {
      cleanup();
      if (!isResolved) {
        isResolved = true;

        // Check if it's an ENOENT error (binary not found)
        if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
          reject(new BinaryNotFoundError(binaryPath));
        } else if ((error as NodeJS.ErrnoException).code === 'EACCES') {
          reject(new SpawnError(`Permission denied: ${binaryPath}`, error));
        } else {
          reject(new SpawnError(`Failed to spawn pdftract: ${error.message}`, error));
        }
      }
    });

    // Handle process exit
    child.on('close', (code) => {
      cleanup();
      if (!isResolved) {
        isResolved = true;

        if (code === 0) {
          // Success - parse stdout as JSON
          try {
            const result = stdoutBuffer.trim() ? JSON.parse(stdoutBuffer) : null;
            resolve(result);
          } catch (error) {
            reject(new Error(`Failed to parse pdftract output as JSON: ${(error as Error).message}`));
          }
        } else {
          // Error - try to parse stderr as JSON
          let errorMessage = stderrBuffer || `pdftract exited with code ${code}`;

          try {
            const errorResponse = JSON.parse(stderrBuffer) as PdftractErrorResponse;
            errorMessage = errorResponse.message || errorResponse.error || errorMessage;
          } catch {
            // Stderr is not JSON, use raw error message
          }

          const error: any = new Error(errorMessage);
          error.exitCode = code;
          error.stderr = stderrBuffer;
          reject(error);
        }
      }
    });

    function cleanup() {
      if (timeoutId) {
        clearTimeout(timeoutId);
        timeoutId = null;
      }
    }
  });
}

/**
 * Spawn pdftract for streaming output (NDJSON)
 *
 * Returns an async iterator that yields parsed JSON objects from stdout.
 *
 * @param args - Command-line arguments to pass to pdftract
 * @param options - Spawn options
 * @returns Async iterable of parsed JSON objects
 */
export async function* spawnPdftractStream<T = any>(
  args: string[],
  options: SpawnOptions = {}
): AsyncGenerator<T> {
  const binaryPath = await resolveBinaryPath();
  const child = spawn(binaryPath, args, {
    stdio: ['pipe', 'pipe', 'pipe'],
    env: { ...process.env, ...options.env },
  });

  let buffer = '';
  let stderrBuffer = '';

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

    if (exitCode !== 0) {
      let errorMessage = stderrBuffer || `pdftract exited with code ${exitCode}`;

      try {
        const errorResponse = JSON.parse(stderrBuffer) as PdftractErrorResponse;
        errorMessage = errorResponse.message || errorResponse.error || errorMessage;
      } catch {
        // Stderr is not JSON
      }

      const error: any = new Error(errorMessage);
      error.exitCode = exitCode;
      error.stderr = stderrBuffer;
      throw error;
    }
  } catch (error) {
    child.kill();
    throw error;
  }
}
