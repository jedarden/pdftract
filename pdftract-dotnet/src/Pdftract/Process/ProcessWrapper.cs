namespace Pdftract.Internal;

using System.Reflection;

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
    /// <exception cref="NotImplementedException">Thrown until implemented in a future bead.</exception>
    public Task<ProcessResult> StartAsync(string[] args)
    {
        throw new NotImplementedException("StartAsync will be implemented in a future bead.");
    }

    /// <summary>
    /// Starts the pdftract process synchronously with the specified arguments.
    /// </summary>
    /// <param name="args">Command-line arguments to pass to the pdftract binary.</param>
    /// <returns>A <see cref="ProcessResult"/> containing the process output.</returns>
    /// <exception cref="NotImplementedException">Thrown until implemented in a future bead.</exception>
    public ProcessResult Start(string[] args)
    {
        throw new NotImplementedException("Start will be implemented in a future bead.");
    }
}

/// <summary>
/// Represents the result of a process execution.
/// </summary>
/// <param name="Stdout">Standard output from the process.</param>
/// <param name="Stderr">Standard error output from the process.</param>
/// <param name="ExitCode">Process exit code.</param>
public record struct ProcessResult(string Stdout, string Stderr, int ExitCode);
