using Xunit;
using Pdftract.Process;
using Pdftract;

namespace Pdftract.Tests.Process;

/// <summary>
/// Unit tests for ErrorParser error JSON parsing and exception mapping.
/// </summary>
public class ErrorParserTests
{
    [Fact]
    public void ParseError_FILE_NOT_FOUND_ReturnsFileNotFoundException()
    {
        // Arrange
        string json = """{"error": {"code": "FILE_NOT_FOUND", "message": "File not found: test.pdf"}}""";
        int exitCode = 1;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<FileNotFoundException>(exception);
        Assert.Equal("FILE_NOT_FOUND", exception.ErrorCode);
        Assert.Equal("File not found: test.pdf", exception.ErrorDetails);
        Assert.Equal(1, exception.ExitCode);
    }

    [Fact]
    public void ParseError_INVALID_FORMAT_ReturnsInvalidFormatException()
    {
        // Arrange
        string json = """{"error": {"code": "INVALID_FORMAT", "message": "Invalid PDF format"}}""";
        int exitCode = 2;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<InvalidFormatException>(exception);
        Assert.Equal("INVALID_FORMAT", exception.ErrorCode);
        Assert.Equal("Invalid PDF format", exception.ErrorDetails);
        Assert.Equal(2, exception.ExitCode);
    }

    [Fact]
    public void ParseError_PERMISSION_DENIED_ReturnsPermissionDeniedException()
    {
        // Arrange
        string json = """{"error": {"code": "PERMISSION_DENIED", "message": "Permission denied"}}""";
        int exitCode = 3;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<PermissionDeniedException>(exception);
        Assert.Equal("PERMISSION_DENIED", exception.ErrorCode);
        Assert.Equal("Permission denied", exception.ErrorDetails);
        Assert.Equal(3, exception.ExitCode);
    }

    [Fact]
    public void ParseError_TIMEOUT_ReturnsTimeoutException()
    {
        // Arrange
        string json = """{"error": {"code": "TIMEOUT", "message": "Operation timed out"}}""";
        int exitCode = 4;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<TimeoutException>(exception);
        Assert.Equal("TIMEOUT", exception.ErrorCode);
        Assert.Equal("Operation timed out", exception.ErrorDetails);
        Assert.Equal(4, exception.ExitCode);
    }

    [Fact]
    public void ParseError_RESOURCE_LIMIT_EXCEEDED_ReturnsResourceLimitExceededException()
    {
        // Arrange
        string json = """{"error": {"code": "RESOURCE_LIMIT_EXCEEDED", "message": "Memory limit exceeded"}}""";
        int exitCode = 5;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<ResourceLimitExceededException>(exception);
        Assert.Equal("RESOURCE_LIMIT_EXCEEDED", exception.ErrorCode);
        Assert.Equal("Memory limit exceeded", exception.ErrorDetails);
        Assert.Equal(5, exception.ExitCode);
    }

    [Fact]
    public void ParseError_ENCODING_ERROR_ReturnsEncodingException()
    {
        // Arrange
        string json = """{"error": {"code": "ENCODING_ERROR", "message": "Text encoding error"}}""";
        int exitCode = 6;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<EncodingException>(exception);
        Assert.Equal("ENCODING_ERROR", exception.ErrorCode);
        Assert.Equal("Text encoding error", exception.ErrorDetails);
        Assert.Equal(6, exception.ExitCode);
    }

    [Fact]
    public void ParseError_VALIDATION_ERROR_ReturnsValidationException()
    {
        // Arrange
        string json = """{"error": {"code": "VALIDATION_ERROR", "message": "Input validation failed"}}""";
        int exitCode = 7;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<ValidationException>(exception);
        Assert.Equal("VALIDATION_ERROR", exception.ErrorCode);
        Assert.Equal("Input validation failed", exception.ErrorDetails);
        Assert.Equal(7, exception.ExitCode);
    }

    [Fact]
    public void ParseError_INTERNAL_ERROR_ReturnsInternalErrorException()
    {
        // Arrange
        string json = """{"error": {"code": "INTERNAL_ERROR", "message": "Internal error occurred"}}""";
        int exitCode = 8;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<InternalErrorException>(exception);
        Assert.Equal("INTERNAL_ERROR", exception.ErrorCode);
        Assert.Equal("Internal error occurred", exception.ErrorDetails);
        Assert.Equal(8, exception.ExitCode);
    }

    [Fact]
    public void ParseError_UnknownErrorCode_ReturnsInternalErrorException()
    {
        // Arrange
        string json = """{"error": {"code": "UNKNOWN_ERROR_CODE", "message": "Some unknown error"}}""";
        int exitCode = 99;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<InternalErrorException>(exception);
        Assert.Contains("UNKNOWN_ERROR_CODE", exception.ErrorDetails);
        Assert.Contains("Some unknown error", exception.ErrorDetails);
        Assert.Equal(99, exception.ExitCode);
    }

    [Fact]
    public void ParseError_MalformedJson_ThrowsArgumentException()
    {
        // Arrange
        string malformedJson = "{invalid json}";
        int exitCode = 1;

        // Act & Assert
        Assert.Throws<ArgumentException>(() => ErrorParser.ParseError(malformedJson, exitCode));
    }

    [Fact]
    public void ParseError_EmptyJson_ThrowsArgumentException()
    {
        // Arrange
        string emptyJson = "";
        int exitCode = 1;

        // Act & Assert
        Assert.ThrowsAny<ArgumentException>(() => ErrorParser.ParseError(emptyJson, exitCode));
    }

    [Fact]
    public void ParseError_WhitespaceJson_ThrowsArgumentException()
    {
        // Arrange
        string whitespaceJson = "   ";
        int exitCode = 1;

        // Act & Assert
        Assert.ThrowsAny<ArgumentException>(() => ErrorParser.ParseError(whitespaceJson, exitCode));
    }

    [Fact]
    public void ParseError_MissingErrorProperty_ReturnsInternalErrorException()
    {
        // Arrange
        string json = """{"not_error": {"code": "FILE_NOT_FOUND", "message": "test"}}""";
        int exitCode = 1;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<InternalErrorException>(exception);
        Assert.Contains("missing 'error' property", exception.ErrorDetails);
    }

    [Fact]
    public void ParseError_MissingCodeProperty_ReturnsInternalErrorException()
    {
        // Arrange
        string json = """{"error": {"message": "test"}}""";
        int exitCode = 1;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<InternalErrorException>(exception);
        Assert.Contains("missing 'code' property", exception.ErrorDetails);
    }

    [Fact]
    public void ParseError_MissingMessageProperty_UsesDefaultMessage()
    {
        // Arrange
        string json = """{"error": {"code": "FILE_NOT_FOUND"}}""";
        int exitCode = 1;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<FileNotFoundException>(exception);
        Assert.Equal("Unknown error", exception.ErrorDetails);
    }

    [Fact]
    public void ParseError_NullExitCode_WithNullableOverload_ReturnsException()
    {
        // Arrange
        string json = """{"error": {"code": "FILE_NOT_FOUND", "message": "File not found"}}""";
        int? exitCode = null;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<FileNotFoundException>(exception);
        Assert.Equal(-1, exception.ExitCode); // Should default to -1
    }

    [Fact]
    public void ParseError_ComplexErrorMessage_PreservesFullMessage()
    {
        // Arrange
        string complexMessage = "Failed to process file: The PDF file is corrupted and cannot be read. Please verify the file integrity.";
        string json = $$"""{"error": {"code": "INVALID_FORMAT", "message": "{{complexMessage}}"}}""";
        int exitCode = 2;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<InvalidFormatException>(exception);
        Assert.Equal(complexMessage, exception.ErrorDetails);
    }

    [Fact]
    public void ParseError_WithSpecialCharactersInMessage_HandlesCorrectly()
    {
        // Arrange
        string message = "Error: File \"test.pdf\" contains special chars: \\n\\t\\r";
        string json = """{"error": {"code": "FILE_NOT_FOUND", "message": "Error: File \"test.pdf\" contains special chars: \\n\\t\\r"}}""";
        int exitCode = 1;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<FileNotFoundException>(exception);
        Assert.NotNull(exception.ErrorDetails);
    }
}
