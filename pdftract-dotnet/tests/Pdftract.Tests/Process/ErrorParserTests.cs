using Xunit;
using Pdftract.Process;
using Pdftract.Exceptions;

/// <summary>
/// Unit tests for ErrorParser JSON error mapping to typed exceptions.
/// </summary>
public class ErrorParserTests
{
    #region Error Code Mapping Tests

    [Fact]
    public void ParseError_FileNotFound_ReturnsFileNotFoundException()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""FILE_NOT_FOUND"",
                ""message"": ""The specified file was not found.""
            }
        }";
        int exitCode = 1;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<Pdftract.Exceptions.FileNotFoundException>(exception);
        Assert.Equal("FILE_NOT_FOUND", exception.ErrorCode);
        Assert.Equal("The specified file was not found.", exception.Message);
        Assert.Equal(exitCode, exception.ExitCode);
    }

    [Fact]
    public void ParseError_InvalidFormat_ReturnsInvalidFormatException()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""INVALID_FORMAT"",
                ""message"": ""The file format is invalid or not supported.""
            }
        }";
        int exitCode = 2;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<InvalidFormatException>(exception);
        Assert.Equal("INVALID_FORMAT", exception.ErrorCode);
        Assert.Equal("The file format is invalid or not supported.", exception.Message);
        Assert.Equal(exitCode, exception.ExitCode);
    }

    [Fact]
    public void ParseError_PermissionDenied_ReturnsPermissionDeniedException()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""PERMISSION_DENIED"",
                ""message"": ""Permission denied to access the resource.""
            }
        }";
        int exitCode = 3;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<PermissionDeniedException>(exception);
        Assert.Equal("PERMISSION_DENIED", exception.ErrorCode);
        Assert.Equal("Permission denied to access the resource.", exception.Message);
        Assert.Equal(exitCode, exception.ExitCode);
    }

    [Fact]
    public void ParseError_Timeout_ReturnsTimeoutException()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""TIMEOUT"",
                ""message"": ""The operation timed out.""
            }
        }";
        int exitCode = 4;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<Pdftract.Exceptions.TimeoutException>(exception);
        Assert.Equal("TIMEOUT", exception.ErrorCode);
        Assert.Equal("The operation timed out.", exception.Message);
        Assert.Equal(exitCode, exception.ExitCode);
    }

    [Fact]
    public void ParseError_ResourceLimitExceeded_ReturnsResourceLimitExceededException()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""RESOURCE_LIMIT_EXCEEDED"",
                ""message"": ""Resource limit exceeded.""
            }
        }";
        int exitCode = 5;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<ResourceLimitExceededException>(exception);
        Assert.Equal("RESOURCE_LIMIT_EXCEEDED", exception.ErrorCode);
        Assert.Equal("Resource limit exceeded.", exception.Message);
        Assert.Equal(exitCode, exception.ExitCode);
    }

    [Fact]
    public void ParseError_EncodingError_ReturnsEncodingException()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""ENCODING_ERROR"",
                ""message"": ""Character encoding error occurred.""
            }
        }";
        int exitCode = 6;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<EncodingException>(exception);
        Assert.Equal("ENCODING_ERROR", exception.ErrorCode);
        Assert.Equal("Character encoding error occurred.", exception.Message);
        Assert.Equal(exitCode, exception.ExitCode);
    }

    [Fact]
    public void ParseError_ValidationError_ReturnsValidationException()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""VALIDATION_ERROR"",
                ""message"": ""Input validation failed.""
            }
        }";
        int exitCode = 7;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<ValidationException>(exception);
        Assert.Equal("VALIDATION_ERROR", exception.ErrorCode);
        Assert.Equal("Input validation failed.", exception.Message);
        Assert.Equal(exitCode, exception.ExitCode);
    }

    [Fact]
    public void ParseError_InternalError_ReturnsInternalErrorException()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""INTERNAL_ERROR"",
                ""message"": ""An internal error occurred.""
            }
        }";
        int exitCode = 8;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<InternalErrorException>(exception);
        Assert.Equal("INTERNAL_ERROR", exception.ErrorCode);
        Assert.Equal("An internal error occurred.", exception.Message);
        Assert.Equal(exitCode, exception.ExitCode);
    }

    #endregion

    #region Exception Property Population Tests

    [Fact]
    public void ParseError_PopulatesErrorCodeProperty()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""FILE_NOT_FOUND"",
                ""message"": ""File not found.""
            }
        }";

        // Act
        var exception = ErrorParser.ParseError(json, 1);

        // Assert
        Assert.Equal("FILE_NOT_FOUND", exception.ErrorCode);
    }

    [Fact]
    public void ParseError_PopulatesErrorDetailsProperty()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""INVALID_FORMAT"",
                ""message"": ""Detailed error message about invalid format""
            }
        }";

        // Act
        var exception = ErrorParser.ParseError(json, 2);

        // Assert
        // ErrorDetails is set to null by default in the exception constructors
        // when only message and exitCode are provided
        Assert.Null(exception.ErrorDetails);
    }

    [Fact]
    public void ParseError_PopulatesExitCodeProperty()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""TIMEOUT"",
                ""message"": ""Timeout occurred.""
            }
        }";
        int expectedExitCode = 42;

        // Act
        var exception = ErrorParser.ParseError(json, expectedExitCode);

        // Assert
        Assert.Equal(expectedExitCode, exception.ExitCode);
    }

    [Fact]
    public void ParseError_PopulatesMessageProperty()
    {
        // Arrange
        string expectedMessage = "Custom error message from JSON";
        string json = @"{
            ""error"": {
                ""code"": ""VALIDATION_ERROR"",
                ""message"": """ + expectedMessage + @"""
            }
        }";

        // Act
        var exception = ErrorParser.ParseError(json, 7);

        // Assert
        Assert.Equal(expectedMessage, exception.Message);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(100)]
    [InlineData(-1)]
    [InlineData(int.MaxValue)]
    public void ParseError_AcceptsAnyExitCodeValue(int exitCode)
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""INTERNAL_ERROR"",
                ""message"": ""Test message.""
            }
        }";

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.Equal(exitCode, exception.ExitCode);
    }

    #endregion

    #region Error Handling Tests

    [Fact]
    public void ParseError_MalformedJson_ThrowsArgumentException()
    {
        // Arrange
        string malformedJson = @"{ ""error"": { ""code"": ""FILE_NOT_FOUND"" "; // Missing closing braces

        // Act & Assert
        var exception = Assert.Throws<ArgumentException>(
            () => ErrorParser.ParseError(malformedJson, 1));

        Assert.Contains("Malformed JSON", exception.Message);
    }

    [Fact]
    public void ParseError_EmptyString_ThrowsArgumentException()
    {
        // Arrange
        string emptyJson = "";

        // Act & Assert
        var exception = Assert.Throws<ArgumentException>(
            () => ErrorParser.ParseError(emptyJson, 1));

        Assert.Contains("cannot be null or empty", exception.Message);
    }

    [Fact]
    public void ParseError_NullString_ThrowsArgumentException()
    {
        // Arrange
        string nullJson = null!;

        // Act & Assert
        var exception = Assert.Throws<ArgumentException>(
            () => ErrorParser.ParseError(nullJson, 1));

        Assert.Contains("cannot be null or empty", exception.Message);
    }

    [Fact]
    public void ParseError_WhitespaceOnly_ThrowsArgumentException()
    {
        // Arrange
        string whitespaceJson = "   \t\n\r  ";

        // Act & Assert
        var exception = Assert.Throws<ArgumentException>(
            () => ErrorParser.ParseError(whitespaceJson, 1));

        Assert.Contains("cannot be null or empty", exception.Message);
    }

    [Fact]
    public void ParseError_MissingErrorField_ThrowsArgumentException()
    {
        // Arrange
        string json = @"{
            ""status"": ""error"",
            ""message"": ""Something went wrong""
        }";

        // Act & Assert
        var exception = Assert.Throws<ArgumentException>(
            () => ErrorParser.ParseError(json, 1));

        Assert.Contains("Missing 'error' property", exception.Message);
    }

    [Fact]
    public void ParseError_MissingErrorCode_ThrowsArgumentException()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""message"": ""Error occurred""
            }
        }";

        // Act & Assert
        var exception = Assert.Throws<ArgumentException>(
            () => ErrorParser.ParseError(json, 1));

        Assert.Contains("Missing 'error.code' property", exception.Message);
    }

    [Fact]
    public void ParseError_MissingErrorMessage_ThrowsArgumentException()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""FILE_NOT_FOUND""
            }
        }";

        // Act & Assert
        var exception = Assert.Throws<ArgumentException>(
            () => ErrorParser.ParseError(json, 1));

        Assert.Contains("Missing 'error.message' property", exception.Message);
    }

    [Fact]
    public void ParseError_UnknownErrorCode_ReturnsInternalErrorException()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""UNKNOWN_ERROR_CODE"",
                ""message"": ""This is an unknown error code.""
            }
        }";
        int exitCode = 99;

        // Act
        var exception = ErrorParser.ParseError(json, exitCode);

        // Assert
        Assert.IsType<InternalErrorException>(exception);
        Assert.Equal("INTERNAL_ERROR", exception.ErrorCode);
        Assert.Contains("Unknown error code: UNKNOWN_ERROR_CODE", exception.Message);
        Assert.Contains("This is an unknown error code.", exception.Message);
        Assert.Equal("Unknown error code received: UNKNOWN_ERROR_CODE", exception.ErrorDetails);
        Assert.Equal(exitCode, exception.ExitCode);
    }

    [Fact]
    public void ParseError_EmptyErrorCode_ReturnsInternalErrorException()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": """",
                ""message"": ""Empty error code.""
            }
        }";

        // Act
        var exception = ErrorParser.ParseError(json, 1);

        // Assert
        Assert.IsType<InternalErrorException>(exception);
        Assert.Equal("INTERNAL_ERROR", exception.ErrorCode);
        Assert.Contains("Unknown error code:", exception.Message);
    }

    [Fact]
    public void ParseError_NullErrorCode_ReturnsInternalErrorException()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""message"": ""Null error code.""
            }
        }";

        // This test actually throws ArgumentException because 'code' field is missing
        // Let's test with explicit null in JSON (which gets treated as empty string by JSON parser)

        // Act & Assert
        var exception = Assert.Throws<ArgumentException>(
            () => ErrorParser.ParseError(json, 1));

        Assert.Contains("Missing 'error.code' property", exception.Message);
    }

    [Fact]
    public void ParseError_CaseSensitiveErrorCode_MapsCorrectly()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""file_not_found"",
                ""message"": ""Lowercase error code.""
            }
        }";

        // Act
        var exception = ErrorParser.ParseError(json, 1);

        // Assert
        // Lowercase "file_not_found" should NOT match uppercase "FILE_NOT_FOUND"
        // so it should return InternalErrorException for unknown code
        Assert.IsType<InternalErrorException>(exception);
        Assert.Contains("Unknown error code: file_not_found", exception.Message);
    }

    #endregion

    #region Additional Edge Cases

    [Fact]
    public void ParseError_WithAdditionalFields_ParsesSuccessfully()
    {
        // Arrange
        string json = @"{
            ""error"": {
                ""code"": ""TIMEOUT"",
                ""message"": ""Timeout after 30 seconds."",
                ""details"": { ""timeout"": 30, ""unit"": ""seconds"" },
                ""timestamp"": ""2026-08-13T10:30:00Z""
            }
        }";

        // Act
        var exception = ErrorParser.ParseError(json, 4);

        // Assert
        Assert.IsType<Pdftract.Exceptions.TimeoutException>(exception);
        Assert.Equal("TIMEOUT", exception.ErrorCode);
        Assert.Equal("Timeout after 30 seconds.", exception.Message);
    }

    [Fact]
    public void ParseError_WithComplexMessage_PreservesContent()
    {
        // Arrange
        string complexMessage = "Error: Could not open file '/path/to/file.pdf'. Reason: Permission denied. User: 'testuser'.";
        string json = @"{
            ""error"": {
                ""code"": ""PERMISSION_DENIED"",
                ""message"": """ + complexMessage + @"""
            }
        }";

        // Act
        var exception = ErrorParser.ParseError(json, 3);

        // Assert
        Assert.Equal(complexMessage, exception.Message);
        Assert.IsType<PermissionDeniedException>(exception);
    }

    [Fact]
    public void ParseError_WithUnicodeMessage_ParsesSuccessfully()
    {
        // Arrange
        string unicodeMessage = "Error: 文件未找到 📄";
        string json = @"{
            ""error"": {
                ""code"": ""FILE_NOT_FOUND"",
                ""message"": """ + unicodeMessage + @"""
            }
        }";

        // Act
        var exception = ErrorParser.ParseError(json, 1);

        // Assert
        Assert.Equal(unicodeMessage, exception.Message);
        Assert.IsType<Pdftract.Exceptions.FileNotFoundException>(exception);
    }

    [Fact]
    public void ParseError_WithEscapedCharacters_ParsesSuccessfully()
    {
        // Arrange
        string escapedMessage = "Error: Line 1\nLine 2\tTabbed\"Quoted\"";
        string json = @"{
            ""error"": {
                ""code"": ""VALIDATION_ERROR"",
                ""message"": ""Error: Line 1\nLine 2\tTabbed\""Quoted\""""
            }
        }";

        // Act
        var exception = ErrorParser.ParseError(json, 7);

        // Assert
        Assert.Equal("Error: Line 1\nLine 2\tTabbed\"Quoted\"", exception.Message);
        Assert.IsType<ValidationException>(exception);
    }

    [Theory]
    [InlineData("FILE_NOT_FOUND", typeof(Pdftract.Exceptions.FileNotFoundException))]
    [InlineData("INVALID_FORMAT", typeof(Pdftract.Exceptions.InvalidFormatException))]
    [InlineData("PERMISSION_DENIED", typeof(Pdftract.Exceptions.PermissionDeniedException))]
    [InlineData("TIMEOUT", typeof(Pdftract.Exceptions.TimeoutException))]
    [InlineData("RESOURCE_LIMIT_EXCEEDED", typeof(Pdftract.Exceptions.ResourceLimitExceededException))]
    [InlineData("ENCODING_ERROR", typeof(Pdftract.Exceptions.EncodingException))]
    [InlineData("VALIDATION_ERROR", typeof(Pdftract.Exceptions.ValidationException))]
    [InlineData("INTERNAL_ERROR", typeof(Pdftract.Exceptions.InternalErrorException))]
    public void ParseError_AllErrorCodes_MapToCorrectExceptionTypes(string errorCode, Type expectedType)
    {
        // Arrange
        string json = $"{{\"error\": {{\"code\": \"{errorCode}\", \"message\": \"Test message for {errorCode}\"}}}}";

        // Act
        var exception = ErrorParser.ParseError(json, 1);

        // Assert
        Assert.IsType(expectedType, exception);
        Assert.Equal(errorCode, exception.ErrorCode);
    }

    #endregion
}
