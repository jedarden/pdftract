using System;
using System.Text.Json;

namespace Pdftract.Process;

/// <summary>
/// Parser for structured JSON error output from the pdftract binary.
/// Maps error codes to typed exception classes for .NET developers.
/// </summary>
public static class ErrorParser
{
    /// <summary>
    /// Parses stderr JSON output from the pdftract binary and maps error codes
    /// to the appropriate exception types.
    /// </summary>
    /// <param name="stderrJson">JSON string from stderr containing error information.</param>
    /// <param name="exitCode">Exit code from the process.</param>
    /// <returns>A typed exception instance matching the error code.</returns>
    /// <exception cref="ArgumentException">Thrown when JSON is malformed or required fields are missing.</exception>
    public static Exceptions.PdftractException ParseError(string stderrJson, int exitCode)
    {
        if (string.IsNullOrWhiteSpace(stderrJson))
        {
            throw new ArgumentException("stderrJson cannot be null or empty.", nameof(stderrJson));
        }

        JsonDocument jsonDoc;
        try
        {
            jsonDoc = JsonDocument.Parse(stderrJson);
        }
        catch (JsonException ex)
        {
            throw new ArgumentException("Malformed JSON in stderr.", nameof(stderrJson), ex);
        }

        JsonElement root = jsonDoc.RootElement;

        if (!root.TryGetProperty("error", out JsonElement errorElement))
        {
            throw new ArgumentException("Missing 'error' property in JSON.", nameof(stderrJson));
        }

        if (!errorElement.TryGetProperty("code", out JsonElement codeElement))
        {
            throw new ArgumentException("Missing 'error.code' property in JSON.", nameof(stderrJson));
        }

        if (!errorElement.TryGetProperty("message", out JsonElement messageElement))
        {
            throw new ArgumentException("Missing 'error.message' property in JSON.", nameof(stderrJson));
        }

        string code = codeElement.GetString() ?? string.Empty;
        string message = messageElement.GetString() ?? string.Empty;

        // Map error codes to exception types
        Exceptions.PdftractException exception = code switch
        {
            "FILE_NOT_FOUND" => new Exceptions.FileNotFoundException(message, exitCode: exitCode),
            "INVALID_FORMAT" => new Exceptions.InvalidFormatException(message, exitCode: exitCode),
            "PERMISSION_DENIED" => new Exceptions.PermissionDeniedException(message, exitCode: exitCode),
            "TIMEOUT" => new Exceptions.TimeoutException(message, exitCode: exitCode),
            "RESOURCE_LIMIT_EXCEEDED" => new Exceptions.ResourceLimitExceededException(message, exitCode: exitCode),
            "ENCODING_ERROR" => new Exceptions.EncodingException(message, exitCode: exitCode),
            "VALIDATION_ERROR" => new Exceptions.ValidationException(message, exitCode: exitCode),
            "INTERNAL_ERROR" => new Exceptions.InternalErrorException(message, exitCode: exitCode),
            _ => new Exceptions.InternalErrorException(
                $"Unknown error code: {code}. Original message: {message}",
                $"Unknown error code received: {code}",
                exitCode)
        };

        return exception;
    }
}
