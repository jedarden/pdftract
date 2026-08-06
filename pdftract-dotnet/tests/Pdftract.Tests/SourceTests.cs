using System.Text.Json;
using Pdftract.Models;
using SourceBase = Pdftract.Models.Source;
using Xunit;

namespace Pdftract.Tests;

/// <summary>
/// Unit tests for Source discriminated union factory methods and JSON deserialization.
/// </summary>
public class SourceTests
{
    /// <summary>
    /// Tests that Source.FilePath.FromPath creates a correct instance with the specified path.
    /// </summary>
    [Fact]
    public void FilePath_FromPath_CreatesCorrectInstance()
    {
        // Arrange
        string expectedPath = "/path/to/document.pdf";

        // Act
        var filePath = SourceBase.FilePath.FromPath(expectedPath);

        // Assert
        Assert.NotNull(filePath);
        Assert.Equal("FilePath", filePath.Type);
        Assert.Equal(expectedPath, filePath.Path);
    }

    /// <summary>
    /// Tests that Source.Base64.FromBase64 creates a correct instance with the specified data.
    /// </summary>
    [Fact]
    public void Base64_FromBase64_CreatesCorrectInstance()
    {
        // Arrange
        string expectedData = "JVBERi0xLjQKJcfsj6IK...";

        // Act
        var base64 = SourceBase.Base64.FromBase64(expectedData);

        // Assert
        Assert.NotNull(base64);
        Assert.Equal("Base64", base64.Type);
        Assert.Equal(expectedData, base64.Data);
    }

    /// <summary>
    /// Tests that Source.Url.FromUrl creates a correct instance with the specified URL.
    /// </summary>
    [Fact]
    public void Url_FromUrl_CreatesCorrectInstance()
    {
        // Arrange
        string expectedUrl = "https://example.com/document.pdf";

        // Act
        var url = SourceBase.Url.FromUrl(expectedUrl);

        // Assert
        Assert.NotNull(url);
        Assert.Equal("Url", url.Type);
        Assert.Equal(expectedUrl, url.UrlValue);
    }

    /// <summary>
    /// Tests that JSON deserialization correctly maps snake_case to PascalCase properties
    /// and deserializes the discriminated union based on the type field.
    /// </summary>
    [Fact]
    public void JsonDeserialization_SnakeCaseToPascalCase_WorksCorrectly()
    {
        // Arrange - snake_case JSON input from Rust binary
        string filePathJson = """{"type":"FilePath","path":"/path/to/document.pdf"}""";
        string base64Json = """{"type":"Base64","data":"JVBERi0xLjQK..."}""";
        string urlJson = """{"type":"Url","url":"https://example.com/document.pdf"}""";

        // Act - deserialize using the configured JsonOptions
        var filePathSource = JsonSerializer.Deserialize<SourceBase>(filePathJson, JsonOptions.Instance);
        var base64Source = JsonSerializer.Deserialize<SourceBase>(base64Json, JsonOptions.Instance);
        var urlSource = JsonSerializer.Deserialize<SourceBase>(urlJson, JsonOptions.Instance);

        // Assert - verify correct types and PascalCase properties
        Assert.NotNull(filePathSource);
        Assert.IsType<SourceBase.FilePath>(filePathSource);
        var filePath = filePathSource as SourceBase.FilePath;
        Assert.Equal("FilePath", filePath!.Type);
        Assert.Equal("/path/to/document.pdf", filePath.Path);

        Assert.NotNull(base64Source);
        Assert.IsType<SourceBase.Base64>(base64Source);
        var base64 = base64Source as SourceBase.Base64;
        Assert.Equal("Base64", base64!.Type);
        Assert.Equal("JVBERi0xLjQK...", base64.Data);

        Assert.NotNull(urlSource);
        Assert.IsType<SourceBase.Url>(urlSource);
        var url = urlSource as SourceBase.Url;
        Assert.Equal("Url", url!.Type);
        Assert.Equal("https://example.com/document.pdf", url.UrlValue);
    }

    /// <summary>
    /// Tests that JSON serialization correctly converts PascalCase properties to snake_case.
    /// </summary>
    [Fact]
    public void JsonSerialization_PascalCaseToSnakeCase_WorksCorrectly()
    {
        // Arrange
        var filePath = Source.FilePath.FromPath("/path/to/document.pdf");

        // Act
        string json = JsonSerializer.Serialize(filePath, JsonOptions.Instance);

        // Assert
        Assert.NotNull(json);
        Assert.Contains("\"type\":\"FilePath\"", json);
        Assert.Contains("\"path\":\"/path/to/document.pdf\"", json);
        // Verify snake_case conversion (no PascalCase in output)
        Assert.DoesNotContain("Path", json);
    }

    /// <summary>
    /// Tests case-insensitive property name matching during deserialization.
    /// </summary>
    [Fact]
    public void JsonDeserialization_CaseInsensitive_WorksCorrectly()
    {
        // Arrange - mixed case property names
        string mixedCaseJson = """{"Type":"FilePath","Path":"/path/to/document.pdf"}""";

        // Act
        var source = JsonSerializer.Deserialize<Source>(mixedCaseJson, JsonOptions.Instance);

        // Assert
        Assert.NotNull(source);
        Assert.IsType<Source.FilePath>(source);
        var filePath = source as Source.FilePath;
        Assert.Equal("/path/to/document.pdf", filePath!.Path);
    }

    /// <summary>
    /// Tests that empty strings don't break the naming policy.
    /// </summary>
    [Fact]
    public void SnakeCaseNamingPolicy_EmptyString_ReturnsEmpty()
    {
        // Arrange
        var policy = new SnakeCaseNamingPolicy();

        // Act
        string result = policy.ConvertName(string.Empty);

        // Assert
        Assert.Equal(string.Empty, result);
    }

    /// <summary>
    /// Tests various PascalCase to snake_case conversions.
    /// </summary>
    [Theory]
    [InlineData("Single", "single")]
    [InlineData("MyProperty", "my_property")]
    [InlineData("XMLParser", "xml_parser")]
    [InlineData("PDFDocument", "pdf_document")]
    [InlineData("GetURLHandler", "get_url_handler")]
    [InlineData("FilePath", "file_path")]
    public void SnakeCaseNamingPolicy_ConvertsCorrectly(string input, string expected)
    {
        // Arrange
        var policy = new SnakeCaseNamingPolicy();

        // Act
        string result = policy.ConvertName(input);

        // Assert
        Assert.Equal(expected, result);
    }
}
