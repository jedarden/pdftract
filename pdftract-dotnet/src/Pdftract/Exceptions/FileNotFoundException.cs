namespace Pdftract;

/// <summary>
/// Exception thrown when a file cannot be found.
/// </summary>
public sealed class FileNotFoundException : PdftractException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="FileNotFoundException"/> class.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    public FileNotFoundException(string errorDetails, int? exitCode = null)
        : base("FILE_NOT_FOUND", errorDetails, exitCode)
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="FileNotFoundException"/> class with an inner exception.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    /// <param name="innerException">The inner exception that caused this exception.</param>
    public FileNotFoundException(string errorDetails, int? exitCode, Exception innerException)
        : base("FILE_NOT_FOUND", errorDetails, exitCode, innerException)
    {
    }
}
