using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;
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
    /// Tests that deserializing a snake_case FilePath payload into the polymorphic base type
    /// selects the FilePath variant and populates its PascalCase property.
    /// </summary>
    [Fact]
    public void JsonDeserialization_FilePathKind_MapsSnakeCaseToPascalCase()
    {
        // Arrange - snake_case wire keys emitted by the pdftract binary
        string json = """{"type":"FilePath","path":"/tmp/invoice.pdf"}""";

        // Act
        var source = JsonSerializer.Deserialize<SourceBase>(json, JsonOptions.Instance);

        // Assert
        var filePath = Assert.IsType<SourceBase.FilePath>(source);
        Assert.Equal("FilePath", filePath.Type);
        Assert.Equal("/tmp/invoice.pdf", filePath.Path);
    }

    /// <summary>
    /// Tests that deserializing a snake_case Base64 payload into the polymorphic base type
    /// selects the Base64 variant and populates its PascalCase property.
    /// </summary>
    [Fact]
    public void JsonDeserialization_Base64Kind_MapsSnakeCaseToPascalCase()
    {
        // Arrange
        string json = """{"type":"Base64","data":"JVBERi0xLjQKJcTg"}""";

        // Act
        var source = JsonSerializer.Deserialize<SourceBase>(json, JsonOptions.Instance);

        // Assert
        var base64 = Assert.IsType<SourceBase.Base64>(source);
        Assert.Equal("Base64", base64.Type);
        Assert.Equal("JVBERi0xLjQKJcTg", base64.Data);
    }

    /// <summary>
    /// Tests that deserializing a snake_case Url payload into the polymorphic base type
    /// selects the Url variant and populates its PascalCase property.
    /// </summary>
    [Fact]
    public void JsonDeserialization_UrlKind_MapsSnakeCaseToPascalCase()
    {
        // Arrange
        string json = """{"type":"Url","url":"https://example.com/invoice.pdf"}""";

        // Act
        var source = JsonSerializer.Deserialize<SourceBase>(json, JsonOptions.Instance);

        // Assert
        var url = Assert.IsType<SourceBase.Url>(source);
        Assert.Equal("Url", url.Type);
        Assert.Equal("https://example.com/invoice.pdf", url.UrlValue);
    }

    /// <summary>
    /// Tests the full round trip for all three Source kinds: a factory-created instance
    /// serializes to snake_case keys with a type discriminator, and deserializing that JSON
    /// back restores the same variant and PascalCase property values.
    /// </summary>
    [Fact]
    public void JsonRoundTrip_AllKinds_PreserveVariantAndProperties()
    {
        // Arrange - declared as the polymorphic base type so the discriminator is emitted
        SourceBase filePath = SourceBase.FilePath.FromPath("/tmp/invoice.pdf");
        SourceBase base64 = SourceBase.Base64.FromBase64("JVBERi0xLjQKJcTg");
        SourceBase url = SourceBase.Url.FromUrl("https://example.com/invoice.pdf");

        // Act
        var filePathRestored = JsonSerializer.Deserialize<SourceBase>(
            JsonSerializer.Serialize(filePath, JsonOptions.Instance), JsonOptions.Instance);
        var base64Restored = JsonSerializer.Deserialize<SourceBase>(
            JsonSerializer.Serialize(base64, JsonOptions.Instance), JsonOptions.Instance);
        var urlRestored = JsonSerializer.Deserialize<SourceBase>(
            JsonSerializer.Serialize(url, JsonOptions.Instance), JsonOptions.Instance);

        // Assert - variant and value both survive the round trip
        var filePathValue = Assert.IsType<SourceBase.FilePath>(filePathRestored);
        Assert.Equal("/tmp/invoice.pdf", filePathValue.Path);

        var base64Value = Assert.IsType<SourceBase.Base64>(base64Restored);
        Assert.Equal("JVBERi0xLjQKJcTg", base64Value.Data);

        var urlValue = Assert.IsType<SourceBase.Url>(urlRestored);
        Assert.Equal("https://example.com/invoice.pdf", urlValue.UrlValue);
    }

    /// <summary>
    /// Tests that JSON serialization correctly converts PascalCase properties to snake_case.
    /// </summary>
    [Fact]
    public void JsonSerialization_PascalCaseToSnakeCase_WorksCorrectly()
    {
        // Arrange - declare as the polymorphic base type; System.Text.Json only emits
        // the type discriminator when the serialized value's declared type is the base type
        SourceBase source = SourceBase.FilePath.FromPath("/path/to/document.pdf");

        // Act
        string json = JsonSerializer.Serialize(source, JsonOptions.Instance);

        // Assert
        Assert.NotNull(json);
        Assert.Contains("\"type\":\"FilePath\"", json);
        Assert.Contains("\"path\":\"/path/to/document.pdf\"", json);
        // Verify snake_case conversion (no PascalCase property names in output).
        // Anchored on the quote/colon so the "FilePath" discriminator value cannot match.
        Assert.DoesNotContain("\"Path\":", json);
        // The Type property must not be written in addition to the discriminator,
        // which would produce two "type" keys in the same object.
        Assert.Equal(1, CountOccurrences(json, "\"type\":"));
    }

    /// <summary>
    /// Tests case-insensitive property name matching during deserialization.
    /// Property keys are matched case-insensitively, but the discriminator key is
    /// matched exactly, so it has to be spelled "type" in the payload.
    /// </summary>
    [Fact]
    public void JsonDeserialization_CaseInsensitive_WorksCorrectly()
    {
        // Arrange - PascalCase property key with the lowercase discriminator key
        string mixedCaseJson = """{"type":"FilePath","Path":"/path/to/document.pdf"}""";

        // Act
        var source = JsonSerializer.Deserialize<SourceBase>(mixedCaseJson, JsonOptions.Instance);

        // Assert
        Assert.NotNull(source);
        var filePath = Assert.IsType<SourceBase.FilePath>(source);
        Assert.Equal("FilePath", filePath.Type);
        Assert.Equal("/path/to/document.pdf", filePath.Path);
    }

    /// <summary>
    /// Tests that an unrecognized type discriminator is rejected rather than silently
    /// producing some other Source variant.
    /// </summary>
    [Fact]
    public void JsonDeserialization_UnknownDiscriminator_Throws()
    {
        // Arrange
        string json = """{"type":"Unknown","path":"/path/to/document.pdf"}""";

        // Act & Assert
        Assert.ThrowsAny<JsonException>(() =>
            JsonSerializer.Deserialize<SourceBase>(json, JsonOptions.Instance));
    }

    /// <summary>
    /// Tests that a Source nested inside another object graph still resolves its
    /// variant and maps snake_case keys, both when reading and when writing.
    /// </summary>
    [Fact]
    public void JsonDeserialization_NestedSource_MapsSnakeCaseToPascalCase()
    {
        // Arrange
        string json = """{"pdf_source":{"type":"Base64","data":"JVBERi0xLjQKJcTg"}}""";

        // Act
        var envelope = JsonSerializer.Deserialize<SourceEnvelope>(json, TestOptions);

        // Assert
        Assert.NotNull(envelope?.PdfSource);
        var nested = Assert.IsType<SourceBase.Base64>(envelope!.PdfSource);
        Assert.Equal("Base64", nested.Type);
        Assert.Equal("JVBERi0xLjQKJcTg", nested.Data);
    }

    /// <summary>
    /// Tests that serializing a nested Source emits snake_case keys at every level.
    /// </summary>
    [Fact]
    public void JsonSerialization_NestedSource_EmitsSnakeCaseKeys()
    {
        // Arrange
        var envelope = new SourceEnvelope { PdfSource = SourceBase.FilePath.FromPath("/tmp/invoice.pdf") };

        // Act
        string json = JsonSerializer.Serialize(envelope, TestOptions);

        // Assert
        Assert.Contains("\"pdf_source\":", json);
        Assert.Contains("\"path\":\"/tmp/invoice.pdf\"", json);
        Assert.DoesNotContain("PdfSource", json);
        Assert.Equal(1, CountOccurrences(json, "\"type\":"));
    }

    /// <summary>
    /// Options that can resolve the test-local SourceEnvelope type. JsonOptions.Instance
    /// resolves types from the AOT source generation context only, which deliberately
    /// does not list test types, so the product context is combined with a reflection
    /// resolver. The naming policy and other settings are inherited from JsonOptions.
    /// </summary>
    private static JsonSerializerOptions TestOptions { get; } = new(JsonOptions.Instance)
    {
        TypeInfoResolver = JsonTypeInfoResolver.Combine(
            PdftractJsonContext.Default,
            new DefaultJsonTypeInfoResolver()),
    };

    /// <summary>
    /// Envelope used to exercise a Source nested one level below the JSON root.
    /// </summary>
    private sealed class SourceEnvelope
    {
        [JsonPropertyName("pdf_source")]
        public SourceBase? PdfSource { get; set; }
    }

    /// <summary>
    /// Counts the occurrences of a substring in a JSON string.
    /// </summary>
    private static int CountOccurrences(string json, string needle)
    {
        int count = 0;
        int index = 0;
        while ((index = json.IndexOf(needle, index, StringComparison.Ordinal)) >= 0)
        {
            count++;
            index += needle.Length;
        }

        return count;
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
    [InlineData("PDFTract", "pdf_tract")]
    [InlineData("GetURLHandler", "get_url_handler")]
    [InlineData("URL", "url")]
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
