namespace Pdftract.Internal;

using System.Reflection;
using System.Diagnostics;

/// <summary>
/// Process wrapper for spawning the pdftract binary with proper binary path resolution.
/// </summary>
public class ProcessWrapper
{
    private readonly CancellationToken _cancellationToken;
    private string? _cachedBinaryPath;

    /// <summary>
    /// Initializes a new instance of the <see cref="ProcessWrapper"/> class.
    /// </summary>
    /// <param name="cancellationToken">Cancellation token for process operations.</param>
    /// <exception cref="FileNotFoundException">Thrown when the pdftract binary cannot be found during construction.</exception>
    public ProcessWrapper(CancellationToken cancellationToken = default)
    {
        _cancellationToken = cancellationToken;
        // Eagerly resolve binary path to validate availability during construction
        _cachedBinaryPath = ResolveBinaryPath();
    }

    /// <summary>
    /// Resolves the path to the pdftract binary.
    /// </summary>
    /// <returns>The full path to the pdftract executable.</returns>
    /// <exception cref="FileNotFoundException">Thrown when the pdftract binary cannot be found.</exception>
    private string ResolveBinaryPath()
    {
        if (_cachedBinaryPath != null)
        {
            return _cachedBinaryPath;
        }

        string binaryPath = string.Empty;
        bool found = false;

        // 1. Check for bundled pdftract binary next to the DLL
        string? assemblyLocation = Assembly.GetExecutingAssembly().Location;
        if (!string.IsNullOrEmpty(assemblyLocation))
        {
            string? assemblyDirectory = Path.GetDirectoryName(assemblyLocation);
            if (!string.IsNullOrEmpty(assemblyDirectory))
            {
                string bundledPath = Path.Combine(assemblyDirectory, GetExecutableName());
                if (File.Exists(bundledPath))
                {
                    binaryPath = bundledPath;
                    found = true;
                }
            }
        }

        // 2. Fall back to PATH environment variable search
        if (!found)
        {
            string? pathEnv = Environment.GetEnvironmentVariable("PATH");
            if (!string.IsNullOrEmpty(pathEnv))
            {
                var pathDirs = pathEnv.Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries);
                foreach (var dir in pathDirs)
                {
                    string pathEntry = Path.Combine(dir, GetExecutableName());
                    if (File.Exists(pathEntry))
                    {
                        binaryPath = pathEntry;
                        found = true;
                        break;
                    }
                }
            }
        }

        // 3. Throw if not found
        if (!found)
        {
            throw new FileNotFoundException(
                $"pdftract binary not found. Searched: assembly directory and PATH environment variable.",
                GetExecutableName());
        }

        _cachedBinaryPath = binaryPath;
        return binaryPath;
    }

    /// <summary>
    /// Gets the platform-specific executable name for pdftract.
    /// </summary>
    /// <returns>The executable filename with extension for the current platform.</returns>
    private static string GetExecutableName()
    {
        if (Environment.OSVersion.Platform == PlatformID.Unix ||
            Environment.OSVersion.Platform == PlatformID.MacOSX ||
            (int)Environment.OSVersion.Platform == 128) // Unix-like on .NET
        {
            return "pdftract";
        }
        else
        {
            return "pdftract.exe";
        }
    }

    /// <summary>
    /// Starts the pdftract process asynchronously with the specified arguments.
    /// </summary>
    /// <param name="args">Command-line arguments to pass to the pdftract binary.</param>
    /// <returns>A <see cref="ProcessResult"/> containing the process output.</returns>
    /// <exception cref="InvalidOperationException">Thrown when process spawning fails.</exception>
    /// <exception cref="OperationCanceledException">Thrown when the cancellation token is canceled.</exception>
    public async Task<ProcessResult> StartAsync(string[] args)
    {
        var binaryPath = ResolveBinaryPath();

        var process = new Process();
        process.StartInfo.FileName = binaryPath;
        process.StartInfo.Arguments = EscapeArguments(args);
        process.StartInfo.UseShellExecute = false;
        process.StartInfo.RedirectStandardOutput = true;
        process.StartInfo.RedirectStandardError = true;
        process.StartInfo.CreateNoWindow = true;

        try
        {
            process.Start();

            // Read stdout asynchronously to avoid deadlocks
            var stdoutTask = process.StandardOutput.ReadToEndAsync(_cancellationToken);

            // Read stderr synchronously (no async API available in .NET for stderr)
            var stderr = process.StandardError.ReadToEnd();

            // Await stdout completion
            var stdout = await stdoutTask.ConfigureAwait(false);

            // Wait for process exit with cancellation support
            await process.WaitForExitAsync(_cancellationToken).ConfigureAwait(false);

            // Check exit code and throw appropriate exceptions on error
            if (process.ExitCode != 0)
            {
                // Try to parse stderr as JSON error, fall back to generic exception
                try
                {
                    var exception = global::Pdftract.Process.ErrorParser.ParseError(stderr, process.ExitCode);
                    throw new PdftractProcessException("pdftract process failed", exception);
                }
                catch (ArgumentException)
                {
                    // If stderr isn't valid JSON, throw a generic exception with the stderr content
                    throw new PdftractProcessException(
                        $"pdftract process failed with exit code {process.ExitCode}: {stderr}");
                }
            }

            return new ProcessResult(
                stdout,
                stderr,
                process.ExitCode
            );
        }
        catch (OperationCanceledException)
        {
            // Kill the entire process tree on cancellation
            try
            {
                if (!process.HasExited)
                {
                    process.Kill(entireProcessTree: true);
                }
            }
            catch
            {
                // Ignore cleanup errors during cancellation
            }
            throw;
        }
        finally
        {
            process.Dispose();
        }
    }

    /// <summary>
    /// Starts the pdftract process synchronously with the specified arguments.
    /// </summary>
    /// <param name="args">Command-line arguments to pass to the pdftract binary.</param>
    /// <returns>A <see cref="ProcessResult"/> containing the process output.</returns>
    /// <exception cref="InvalidOperationException">Thrown when process spawning fails.</exception>
    /// <exception cref="OperationCanceledException">Thrown when the cancellation token is canceled.</exception>
    public ProcessResult Start(string[] args)
    {
        var binaryPath = ResolveBinaryPath();

        var process = new Process();
        process.StartInfo.FileName = binaryPath;
        process.StartInfo.Arguments = EscapeArguments(args);
        process.StartInfo.UseShellExecute = false;
        process.StartInfo.RedirectStandardOutput = true;
        process.StartInfo.RedirectStandardError = true;
        process.StartInfo.CreateNoWindow = true;

        try
        {
            process.Start();

            // Read stdout first to avoid deadlocks
            var stdout = process.StandardOutput.ReadToEnd();

            // Then read stderr
            var stderr = process.StandardError.ReadToEnd();

            // Wait for process to exit
            process.WaitForExit();

            // Check exit code and throw appropriate exceptions on error
            if (process.ExitCode != 0)
            {
                // Try to parse stderr as JSON error, fall back to generic exception
                try
                {
                    var exception = global::Pdftract.Process.ErrorParser.ParseError(stderr, process.ExitCode);
                    throw new PdftractProcessException("pdftract process failed", exception);
                }
                catch (ArgumentException)
                {
                    // If stderr isn't valid JSON, throw a generic exception with the stderr content
                    throw new PdftractProcessException(
                        $"pdftract process failed with exit code {process.ExitCode}: {stderr}");
                }
            }

            return new ProcessResult(
                stdout,
                stderr,
                process.ExitCode
            );
        }
        catch (OperationCanceledException)
        {
            // Kill the entire process tree on cancellation
            try
            {
                if (!process.HasExited)
                {
                    process.Kill(entireProcessTree: true);
                }
            }
            catch
            {
                // Ignore cleanup errors during cancellation
            }
            throw;
        }
        finally
        {
            process.Dispose();
        }
    }

    /// <summary>
    /// Escapes command-line arguments for safe use in ProcessStartInfo.Arguments.
    /// </summary>
    /// <param name="args">Array of command-line arguments.</param>
    /// <returns>A properly escaped argument string.</returns>
    private static string EscapeArguments(string[] args)
    {
        if (args == null || args.Length == 0)
        {
            return string.Empty;
        }

        var escapedArgs = new System.Text.StringBuilder();

        foreach (var arg in args)
        {
            if (escapedArgs.Length > 0)
            {
                escapedArgs.Append(' ');
            }

            // Escape arguments containing spaces, quotes, or special characters
            if (arg.Contains(' ') || arg.Contains('\t') || arg.Contains('"') || arg.Contains('\\'))
            {
                escapedArgs.Append('"');

                // Escape backslashes and quotes
                for (int i = 0; i < arg.Length; i++)
                {
                    char c = arg[i];
                    if (c == '\\')
                    {
                        // Count consecutive backslashes
                        int backslashCount = 1;
                        while (i + backslashCount < arg.Length && arg[i + backslashCount] == '\\')
                        {
                            backslashCount++;
                        }

                        // If before a quote or at end, double the backslashes
                        if (i + backslashCount < arg.Length && arg[i + backslashCount] == '"')
                        {
                            escapedArgs.Append('\\', backslashCount * 2);
                            i += backslashCount - 1; // Move to last backslash
                        }
                        else
                        {
                            escapedArgs.Append('\\', backslashCount);
                            i += backslashCount - 1; // Move to last backslash
                        }
                    }
                    else if (c == '"')
                    {
                        escapedArgs.Append("\\\"");
                    }
                    else
                    {
                        escapedArgs.Append(c);
                    }
                }

                escapedArgs.Append('"');
            }
            else
            {
                escapedArgs.Append(arg);
            }
        }

        return escapedArgs.ToString();
    }
}

/// <summary>
/// Represents the result of a process execution.
/// </summary>
/// <param name="Stdout">Standard output from the process.</param>
/// <param name="Stderr">Standard error output from the process.</param>
/// <param name="ExitCode">Process exit code.</param>
public record struct ProcessResult(string Stdout, string Stderr, int ExitCode);

/// <summary>
/// Exception thrown when the pdftract process fails.
/// Wraps either a typed PdftractException or a generic process failure.
/// </summary>
public class PdftractProcessException : Exception
{
    /// <summary>
    /// The underlying PdftractException, if stderr was valid JSON.
    /// </summary>
    public global::Pdftract.Exceptions.PdftractException? UnderlyingException { get; }

    /// <summary>
    /// Initializes a new instance of the <see cref="PdftractProcessException"/> class
    /// with a wrapped PdftractException.
    /// </summary>
    /// <param name="message">The error message.</param>
    /// <param name="innerException">The underlying PdftractException.</param>
    public PdftractProcessException(string message, global::Pdftract.Exceptions.PdftractException innerException)
        : base(message, innerException)
    {
        UnderlyingException = innerException;
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="PdftractProcessException"/> class
    /// with a generic error message (when stderr isn't valid JSON).
    /// </summary>
    /// <param name="message">The error message.</param>
    public PdftractProcessException(string message)
        : base(message)
    {
        UnderlyingException = null;
    }
}
