using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using Pdftract.Models;

namespace Pdftract;

/// <summary>
/// pdftract SDK client for .NET.
/// </summary>
public sealed partial class Pdftract : IAsyncDisposable, IDisposable
{
    private readonly string _binaryPath;
    private readonly JsonSerializerOptions _jsonOptions;

    /// <summary>
    /// Creates a new Pdftract client with the specified binary path.
    /// </summary>
    /// <param name="binaryPath">Path to the pdftract binary. If null, searches PATH.</param>
    public Pdftract(string? binaryPath = null)
    {
        _binaryPath = FindBinary(binaryPath);
        _jsonOptions = PdftractJsonContext.Default.Options;
    }

    /// <summary>
    /// Extracts structured data from a PDF.
    /// </summary>
    public async Task<Document> ExtractAsync(
        Source source,
        ExtractOptions? options = null,
        CancellationToken cancellationToken = default)
    {
        var args = BuildArgs("extract", "--json", source, options);
        var json = await InvokeAsync(source, args, cancellationToken);
        return JsonSerializer.Deserialize(json, PdftractJsonContext.Default.Document)
            ?? throw new JsonException("Failed to deserialize Document");
    }

    /// <summary>
    /// Extracts plain text from a PDF.
    /// </summary>
    public async Task<string> ExtractTextAsync(
        Source source,
        ExtractOptions? options = null,
        CancellationToken cancellationToken = default)
    {
        var args = BuildArgs("extract", "--text", source, options);
        return await InvokeAsync(source, args, cancellationToken);
    }

    /// <summary>
    /// Extracts markdown-formatted text from a PDF.
    /// </summary>
    public async Task<string> ExtractMarkdownAsync(
        Source source,
        ExtractOptions? options = null,
        CancellationToken cancellationToken = default)
    {
        var args = BuildArgs("extract", "--md", source, options);
        return await InvokeAsync(source, args, cancellationToken);
    }

    /// <summary>
    /// Extracts pages from a PDF as a stream.
    /// </summary>
    public async IAsyncEnumerable<Page> ExtractStreamAsync(
        Source source,
        ExtractOptions? options = null,
        [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        var args = BuildArgs("extract", "--ndjson", source, options);
        await foreach (var line in InvokeStreamAsync(source, args, cancellationToken))
        {
            var page = JsonSerializer.Deserialize(line, PdftractJsonContext.Default.Page)
                ?? throw new JsonException("Failed to deserialize Page");
            yield return page;
        }
    }

    /// <summary>
    /// Searches for a pattern in a PDF.
    /// </summary>
    public async IAsyncEnumerable<Match> SearchAsync(
        Source source,
        string pattern,
        SearchOptions? options = null,
        [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        var args = BuildArgs("grep", pattern, source, options);
        await foreach (var line in InvokeStreamAsync(source, args, cancellationToken))
        {
            var match = JsonSerializer.Deserialize(line, PdftractJsonContext.Default.Match)
                ?? throw new JsonException("Failed to deserialize Match");
            yield return match;
        }
    }

    /// <summary>
    /// Extracts metadata from a PDF.
    /// </summary>
    public async Task<Metadata> GetMetadataAsync(
        Source source,
        ExtractOptions? options = null,
        CancellationToken cancellationToken = default)
    {
        var args = BuildArgs("extract", "--metadata-only", source, options);
        var json = await InvokeAsync(source, args, cancellationToken);

        var result = JsonSerializer.Deserialize(json, PdftractJsonContext.Default.Document);
        if (result is null) throw new JsonException("Failed to deserialize Document");
        return result.Metadata;
    }

    /// <summary>
    /// Computes the fingerprint hash of a PDF.
    /// </summary>
    public async Task<Fingerprint> HashAsync(
        Source source,
        HashOptions? options = null,
        CancellationToken cancellationToken = default)
    {
        var args = new List<string> { "hash" };
        args.AddRange(source.ToArgs());
        if (options != null)
        {
            args.AddRange(options.ToArgs());
        }

        var json = await InvokeAsync(source, args, cancellationToken);
        return JsonSerializer.Deserialize(json, PdftractJsonContext.Default.Fingerprint)
            ?? throw new JsonException("Failed to deserialize Fingerprint");
    }

    /// <summary>
    /// Classifies a PDF document.
    /// </summary>
    public async Task<Classification> ClassifyAsync(
        Source source,
        CancellationToken cancellationToken = default)
    {
        var args = new List<string> { "classify" };
        args.AddRange(source.ToArgs());

        var json = await InvokeAsync(source, args, cancellationToken);
        return JsonSerializer.Deserialize(json, PdftractJsonContext.Default.Classification)
            ?? throw new JsonException("Failed to deserialize Classification");
    }

    /// <summary>
    /// Verifies a cryptographic receipt for a PDF.
    /// </summary>
    public async Task<bool> VerifyReceiptAsync(
        string path,
        Receipt receipt,
        CancellationToken cancellationToken = default)
    {
        var receiptPath = path + ".receipt.json";
        var receiptJson = JsonSerializer.Serialize(receipt, PdftractJsonContext.Default.Receipt);
        await File.WriteAllTextAsync(receiptPath, receiptJson, cancellationToken);

        try
        {
            var args = new List<string> { "verify-receipt", path, receiptPath };
            await InvokeAsync(null, args, cancellationToken);
            return true;
        }
        catch (ReceiptVerifyException)
        {
            return false;
        }
        finally
        {
            try
            {
                if (File.Exists(receiptPath))
                {
                    File.Delete(receiptPath);
                }
            }
            catch
            {
                // Ignore cleanup errors
            }
        }
    }

    /// <summary>
    /// Returns the path to the pdftract binary.
    /// </summary>
    public string BinaryPath => _binaryPath;

    /// <summary>
    /// Returns the pdftract binary version.
    /// </summary>
    public async Task<string> GetVersionAsync(CancellationToken cancellationToken = default)
    {
        var args = new List<string> { "--version" };
        return await InvokeAsync(null, args, cancellationToken);
    }

    private static List<string> BuildArgs(
        string command,
        string flag,
        Source source,
        ExtractOptions? options)
    {
        var args = new List<string> { command, flag };
        args.AddRange(source.ToArgs());
        if (options != null)
        {
            args.AddRange(options.ToArgs());
        }
        return args;
    }

    private static List<string> BuildArgs(
        string command,
        string pattern,
        Source source,
        SearchOptions? options)
    {
        var args = new List<string> { command, pattern };
        args.AddRange(source.ToArgs());
        if (options != null)
        {
            args.AddRange(options.ToArgs());
        }
        return args;
    }

    private async Task<string> InvokeAsync(
        Source? source,
        List<string> args,
        CancellationToken cancellationToken)
    {
        using var process = new System.Diagnostics.Process();
        process.StartInfo = new ProcessStartInfo
        {
            FileName = _binaryPath,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false
        };
        foreach (var arg in args)
        {
            process.StartInfo.ArgumentList.Add(arg);
        }

        var output = new StringBuilder();
        var error = new StringBuilder();

        process.OutputDataReceived += (_, e) => { if (e.Data != null) { output.AppendLine(e.Data); } };
        process.ErrorDataReceived += (_, e) => { if (e.Data != null) { error.AppendLine(e.Data); } };

        var tcs = new TaskCompletionSource<string>();

        cancellationToken.Register(() =>
        {
            try
            {
                process.Kill(entireProcessTree: true);
                tcs.TrySetCanceled(cancellationToken);
            }
            catch
            {
                // Ignore
            }
        });

        process.Exited += (_, _) =>
        {
            try
            {
                if (cancellationToken.IsCancellationRequested)
                {
                    tcs.TrySetCanceled(cancellationToken);
                    return;
                }

                if (process.ExitCode != 0)
                {
                    var exception = PdftractException.FromExitCode(process.ExitCode, error.ToString());
                    tcs.TrySetException(exception);
                    return;
                }

                tcs.TrySetResult(output.ToString());
            }
            catch (Exception ex)
            {
                tcs.TrySetException(ex);
            }
        };

        process.EnableRaisingEvents = true;

        if (!process.Start())
        {
            source?.Dispose();
            throw new InvalidOperationException("Failed to start pdftract process");
        }

        process.BeginOutputReadLine();
        process.BeginErrorReadLine();

        try
        {
            var result = await tcs.Task;
            return result;
        }
        finally
        {
            source?.Dispose();
        }
    }

    private async IAsyncEnumerable<string> InvokeStreamAsync(
        Source source,
        List<string> args,
        [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken cancellationToken)
    {
        using var process = new System.Diagnostics.Process();
        process.StartInfo = new ProcessStartInfo
        {
            FileName = _binaryPath,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false
        };
        foreach (var arg in args)
        {
            process.StartInfo.ArgumentList.Add(arg);
        }

        var error = new StringBuilder();
        var processExitCode = 0;
        var processExited = false;

        process.ErrorDataReceived += (_, e) => { if (e.Data != null) { error.AppendLine(e.Data); } };

        cancellationToken.Register(() =>
        {
            try
            {
                process.Kill(entireProcessTree: true);
            }
            catch
            {
                // Ignore
            }
        });

        process.Exited += (_, _) =>
        {
            processExitCode = process.ExitCode;
            processExited = true;
        };

        process.EnableRaisingEvents = true;

        if (!process.Start())
        {
            source.Dispose();
            throw new InvalidOperationException("Failed to start pdftract process");
        }

        try
        {
            using var reader = process.StandardOutput;
            process.BeginErrorReadLine();

            string? line;
            while ((line = await reader.ReadLineAsync(cancellationToken)) != null)
            {
                if (!string.IsNullOrWhiteSpace(line))
                {
                    yield return line;
                }
            }

            process.WaitForExit();

            if (cancellationToken.IsCancellationRequested)
            {
                throw new OperationCanceledException("pdftract cancelled", cancellationToken);
            }

            if (processExitCode != 0)
            {
                throw PdftractException.FromExitCode(processExitCode, error.ToString());
            }
        }
        finally
        {
            source.Dispose();
        }
    }

    private static string FindBinary(string? path)
    {
        var binaryPath = path;

        if (string.IsNullOrEmpty(binaryPath))
        {
            // Search in PATH
            var pathEnv = Environment.GetEnvironmentVariable("PATH");
            if (pathEnv != null)
            {
                var separators = RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
                    ? new[] { ';' }
                    : new[] { ':' };

                foreach (var dir in pathEnv.Split(separators, StringSplitOptions.RemoveEmptyEntries))
                {
                    var candidate = Path.Combine(dir, "pdftract");
                    if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
                    {
                        candidate += ".exe";
                    }

                    if (File.Exists(candidate))
                    {
                        binaryPath = candidate;
                        break;
                    }
                }
            }
        }

        if (string.IsNullOrEmpty(binaryPath))
        {
            throw new FileNotFoundException(
                "pdftract binary not found. Please install pdftract or provide the binary path.");
        }

        if (!File.Exists(binaryPath))
        {
            throw new FileNotFoundException($"pdftract binary not found at {binaryPath}");
        }

        return binaryPath;
    }

    public void Dispose()
    {
        // No unmanaged resources to dispose
    }

    public async ValueTask DisposeAsync()
    {
        // No unmanaged resources to dispose
        await Task.CompletedTask;
    }
}
