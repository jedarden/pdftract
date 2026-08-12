namespace Pdftract;

/// <summary>
/// Exception thrown when a PDF file is malformed or has an unsupported format.
/// </summary>
public sealed class InvalidFormatException : PdftractException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="InvalidFormatException"/> class.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    public InvalidFormatException(string errorDetails, int? exitCode = null)
        : base("INVALID_FORMAT", errorDetails, exitCode)
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="InvalidFormatException"/> class with an inner exception.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    /// <param name="innerException">The inner exception that caused this exception.</param>
    public InvalidFormatException(string errorDetails, int? exitCode, Exception innerException)
        : base("INVALID_FORMAT", errorDetails, exitCode, innerException)
    {
    }
}
