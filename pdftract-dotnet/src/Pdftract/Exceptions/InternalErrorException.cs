namespace Pdftract;

/// <summary>
/// Exception thrown when unexpected internal errors occur.
/// </summary>
public sealed class InternalErrorException : PdftractException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="InternalErrorException"/> class.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    public InternalErrorException(string errorDetails, int? exitCode = null)
        : base("INTERNAL_ERROR", errorDetails, exitCode)
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="InternalErrorException"/> class with an inner exception.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    /// <param name="innerException">The inner exception that caused this exception.</param>
    public InternalErrorException(string errorDetails, int? exitCode, Exception innerException)
        : base("INTERNAL_ERROR", errorDetails, exitCode, innerException)
    {
    }
}
