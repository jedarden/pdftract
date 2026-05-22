using System.Diagnostics.CodeAnalysis;

namespace Pdftract;

/// <summary>
/// Base exception for all pdftract errors.
/// </summary>
public abstract class PdftractException : Exception
{
    /// <summary>
    /// The exit code from the pdftract binary.
    /// </summary>
    public int ExitCode { get; }

    protected PdftractException(int exitCode, string? message) : base(message)
    {
        ExitCode = exitCode;
    }

    protected PdftractException(int exitCode, string? message, Exception? innerException) 
        : base(message, innerException)
    {
        ExitCode = exitCode;
    }

    /// <summary>
    /// Maps an exit code and stderr to the appropriate exception type.
    /// </summary>
    public static PdftractException FromExitCode(int exitCode, string stderr)
    {
        var message = string.IsNullOrEmpty(stderr) ? "unknown error" : stderr;

        return exitCode switch
        {
            2 => new CorruptPdfException(exitCode, message),
            3 => new EncryptionException(exitCode, message),
            4 => new SourceUnreachableException(exitCode, message),
            5 => new RemoteFetchInterruptedException(exitCode, message),
            6 => new TlsException(exitCode, message),
            10 => new ReceiptVerifyException(exitCode, message),
            _ => new UnknownPdftractException(exitCode, message)
        };
    }
}

/// <summary>
/// Unknown pdftract error (unexpected exit code).
/// </summary>
public sealed class UnknownPdftractException : PdftractException
{
    public UnknownPdftractException(int exitCode, string? message) 
        : base(exitCode, message) { }
}

/// <summary>
/// Corrupt PDF error (exit code 2).
/// </summary>
public sealed class CorruptPdfException : PdftractException
{
    public CorruptPdfException(int exitCode, string? message) 
        : base(exitCode, message) { }
}

/// <summary>
/// Encryption error (exit code 3) — password missing or incorrect.
/// </summary>
public sealed class EncryptionException : PdftractException
{
    public EncryptionException(int exitCode, string? message) 
        : base(exitCode, message) { }
}

/// <summary>
/// Source unreachable error (exit code 4) — file or URL cannot be read.
/// </summary>
public sealed class SourceUnreachableException : PdftractException
{
    public SourceUnreachableException(int exitCode, string? message) 
        : base(exitCode, message) { }
}

/// <summary>
/// Remote fetch interrupted error (exit code 5) — network connection failed.
/// </summary>
public sealed class RemoteFetchInterruptedException : PdftractException
{
    public RemoteFetchInterruptedException(int exitCode, string? message) 
        : base(exitCode, message) { }
}

/// <summary>
/// TLS/certificate error (exit code 6) — certificate validation failed.
/// </summary>
public sealed class TlsException : PdftractException
{
    public TlsException(int exitCode, string? message) 
        : base(exitCode, message) { }
}

/// <summary>
/// Receipt verification failure (exit code 10).
/// </summary>
public sealed class ReceiptVerifyException : PdftractException
{
    public ReceiptVerifyException(int exitCode, string? message) 
        : base(exitCode, message) { }
}
