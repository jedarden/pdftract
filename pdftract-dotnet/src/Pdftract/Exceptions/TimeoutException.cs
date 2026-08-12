namespace Pdftract;

/// <summary>
/// Exception thrown when an operation times out.
/// </summary>
public sealed class TimeoutException : PdftractException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="TimeoutException"/> class.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    public TimeoutException(string errorDetails, int? exitCode = null)
        : base("TIMEOUT", errorDetails, exitCode)
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="TimeoutException"/> class with an inner exception.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    /// <param name="innerException">The inner exception that caused this exception.</param>
    public TimeoutException(string errorDetails, int? exitCode, Exception innerException)
        : base("TIMEOUT", errorDetails, exitCode, innerException)
    {
    }
}
