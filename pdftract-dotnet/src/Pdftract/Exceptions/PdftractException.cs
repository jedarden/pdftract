namespace Pdftract;

/// <summary>
/// Base exception for all pdftract errors.
/// </summary>
public abstract class PdftractException : Exception
{
    /// <summary>
    /// The error code from the pdftract binary.
    /// </summary>
    public string ErrorCode { get; }

    /// <summary>
    /// Detailed error information from the pdftract binary.
    /// </summary>
    public string ErrorDetails { get; }

    /// <summary>
    /// The exit code from the pdftract binary, if available.
    /// </summary>
    public int? ExitCode { get; }

    /// <summary>
    /// Initializes a new instance of the <see cref="PdftractException"/> class.
    /// </summary>
    /// <param name="errorCode">The error code from the pdftract binary.</param>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    protected PdftractException(string errorCode, string errorDetails, int? exitCode)
        : base(errorDetails)
    {
        ErrorCode = errorCode;
        ErrorDetails = errorDetails;
        ExitCode = exitCode;
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="PdftractException"/> class with an inner exception.
    /// </summary>
    /// <param name="errorCode">The error code from the pdftract binary.</param>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    /// <param name="innerException">The inner exception that caused this exception.</param>
    protected PdftractException(string errorCode, string errorDetails, int? exitCode, Exception innerException)
        : base(errorDetails, innerException)
    {
        ErrorCode = errorCode;
        ErrorDetails = errorDetails;
        ExitCode = exitCode;
    }
}
