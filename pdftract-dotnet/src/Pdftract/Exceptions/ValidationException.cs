namespace Pdftract;

/// <summary>
/// Exception thrown when input validation failures occur.
/// </summary>
public sealed class ValidationException : PdftractException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="ValidationException"/> class.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    public ValidationException(string errorDetails, int? exitCode = null)
        : base("VALIDATION_ERROR", errorDetails, exitCode)
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="ValidationException"/> class with an inner exception.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    /// <param name="innerException">The inner exception that caused this exception.</param>
    public ValidationException(string errorDetails, int? exitCode, Exception innerException)
        : base("VALIDATION_ERROR", errorDetails, exitCode, innerException)
    {
    }
}
