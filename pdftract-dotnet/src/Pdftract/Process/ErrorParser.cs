namespace Pdftract.Internal;

using System.Text.Json;

/// <summary>
/// Parses error output from the pdftract binary and maps to appropriate exception types.
/// </summary>
public static class ErrorParser
{
    /// <summary>
    /// Parses error JSON from stderr and returns the appropriate exception.
    /// </summary>
    /// <param name="stderrJson">The JSON string from stderr.</param>
    /// <param name="exitCode">The exit code from the pdftract binary.</param>
    /// <returns>A <see cref="global::Pdftract.PdftractException"/> instance representing the error.</returns>
    /// <exception cref="ArgumentException">Thrown when the JSON is malformed.</exception>
    public static global::Pdftract.PdftractException ParseError(string stderrJson, int exitCode)
    {
        if (string.IsNullOrWhiteSpace(stderrJson))
        {
            return new global::Pdftract.InternalErrorException("No error details provided", exitCode);
        }

        try
        {
            // Parse the JSON
            using JsonDocument doc = JsonDocument.Parse(stderrJson);
            JsonElement root = doc.RootElement;

            // Check if it has an "error" property
            if (!root.TryGetProperty("error", out JsonElement errorElement))
            {
                // If no error property, treat as internal error
                return new global::Pdftract.InternalErrorException($"Invalid error format: missing 'error' property", exitCode);
            }

            // Get the error code
            if (!errorElement.TryGetProperty("code", out JsonElement codeElement))
            {
                return new global::Pdftract.InternalErrorException($"Invalid error format: missing 'code' property", exitCode);
            }

            string errorCode = codeElement.GetString() ?? string.Empty;

            // Get the error message/details
            string errorDetails = "Unknown error";
            if (errorElement.TryGetProperty("message", out JsonElement messageElement))
            {
                errorDetails = messageElement.GetString() ?? errorDetails;
            }

            // Map error codes to exception types
            return errorCode switch
            {
                "FILE_NOT_FOUND" => new global::Pdftract.FileNotFoundException(errorDetails, exitCode),
                "INVALID_FORMAT" => new global::Pdftract.InvalidFormatException(errorDetails, exitCode),
                "PERMISSION_DENIED" => new global::Pdftract.PermissionDeniedException(errorDetails, exitCode),
                "TIMEOUT" => new global::Pdftract.TimeoutException(errorDetails, exitCode),
                "RESOURCE_LIMIT_EXCEEDED" => new global::Pdftract.ResourceLimitExceededException(errorDetails, exitCode),
                "ENCODING_ERROR" => new global::Pdftract.EncodingException(errorDetails, exitCode),
                "VALIDATION_ERROR" => new global::Pdftract.ValidationException(errorDetails, exitCode),
                "INTERNAL_ERROR" => new global::Pdftract.InternalErrorException(errorDetails, exitCode),
                _ => new global::Pdftract.InternalErrorException($"Unknown error code: {errorCode}. {errorDetails}", exitCode)
            };
        }
        catch (JsonException ex)
        {
            // JSON parsing failed - this is a malformed error response
            throw new ArgumentException("Failed to parse error JSON from stderr", ex);
        }
    }

    /// <summary>
    /// Parses error JSON from stderr and returns the appropriate exception (nullable exit code overload).
    /// </summary>
    /// <param name="stderrJson">The JSON string from stderr.</param>
    /// <param name="exitCode">The exit code from the pdftract binary, if available.</param>
    /// <returns>A <see cref="global::Pdftract.PdftractException"/> instance representing the error.</returns>
    /// <exception cref="ArgumentException">Thrown when the JSON is malformed.</exception>
    public static global::Pdftract.PdftractException ParseError(string stderrJson, int? exitCode)
    {
        return ParseError(stderrJson, exitCode ?? -1);
    }
}
