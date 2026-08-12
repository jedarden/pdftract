namespace Pdftract;

/// <summary>
/// Exception thrown when memory or CPU limits are exceeded.
/// </summary>
public sealed class ResourceLimitExceededException : PdftractException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="ResourceLimitExceededException"/> class.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    public ResourceLimitExceededException(string errorDetails, int? exitCode = null)
        : base("RESOURCE_LIMIT_EXCEEDED", errorDetails, exitCode)
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="ResourceLimitExceededException"/> class with an inner exception.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    /// <param name="innerException">The inner exception that caused this exception.</param>
    public ResourceLimitExceededException(string errorDetails, int? exitCode, Exception innerException)
        : base("RESOURCE_LIMIT_EXCEEDED", errorDetails, exitCode, innerException)
    {
    }
}
