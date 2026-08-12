namespace Pdftract;

/// <summary>
/// Exception thrown when filesystem or permission errors occur.
/// </summary>
public sealed class PermissionDeniedException : PdftractException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="PermissionDeniedException"/> class.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    public PermissionDeniedException(string errorDetails, int? exitCode = null)
        : base("PERMISSION_DENIED", errorDetails, exitCode)
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="PermissionDeniedException"/> class with an inner exception.
    /// </summary>
    /// <param name="errorDetails">Detailed error information.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    /// <param name="innerException">The inner exception that caused this exception.</param>
    public PermissionDeniedException(string errorDetails, int? exitCode, Exception innerException)
        : base("PERMISSION_DENIED", errorDetails, exitCode, innerException)
    {
    }
}
