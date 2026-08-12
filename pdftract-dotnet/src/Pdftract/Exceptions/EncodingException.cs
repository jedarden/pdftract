namespace Pdftract;

/// <summary>
/// Exception thrown when text encoding issues occur.
/// </summary>
public sealed class EncodingException : PdftractException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="EncodingException"/> class.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    public EncodingException(string errorDetails, int? exitCode = null)
        : base("ENCODING_ERROR", errorDetails, exitCode)
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="EncodingException"/> class with an inner exception.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    /// <param name="innerException">The inner exception that caused this exception.</param>
    public EncodingException(string errorDetails, int? exitCode, Exception innerException)
        : base("ENCODING_ERROR", errorDetails, exitCode, innerException)
    {
    }
}
