using System.Text.Json;
using Pdftract.Models;
using Xunit;

namespace Pdftract.Tests;

public class DiagnosticSerializationTests
{
    [Fact]
    public void Diagnostic_PreservesCanonicalFieldsAndOmitsAbsentContext()
    {
        const string json = """
            {"code":"STREAM_DECODE_ERROR","message":"zlib stream truncated mid-inflation","severity":"warning","page_index":3,"location":{"object_number":42,"generation_number":7},"hint":"Inspect the source PDF for corrupt stream data"}
            """;

        var diagnostic = JsonSerializer.Deserialize(
            json, PdftractJsonContext.Default.Error);

        Assert.NotNull(diagnostic);
        Assert.Equal("STREAM_DECODE_ERROR", diagnostic.Code);
        Assert.Equal("warning", diagnostic.Severity);
        Assert.Equal(3, diagnostic.PageIndex);
        Assert.Equal(42, diagnostic.Location!.ObjectNumber);
        Assert.Equal(7, diagnostic.Location.GenerationNumber);
        Assert.Equal("Inspect the source PDF for corrupt stream data", diagnostic.Hint);

        var encoded = JsonSerializer.Serialize(
            diagnostic, PdftractJsonContext.Default.Error);
        using var document = JsonDocument.Parse(encoded);
        Assert.Equal(3, document.RootElement.GetProperty("page_index").GetInt32());
        Assert.Equal(42, document.RootElement.GetProperty("location")
            .GetProperty("object_number").GetInt32());

        var bare = new Error
        {
            Code = "XREF_REPAIRED",
            Message = "repaired",
            Severity = "info"
        };
        using var bareDocument = JsonDocument.Parse(JsonSerializer.Serialize(
            bare, PdftractJsonContext.Default.Error));
        Assert.False(bareDocument.RootElement.TryGetProperty("page_index", out _));
        Assert.False(bareDocument.RootElement.TryGetProperty("location", out _));
        Assert.False(bareDocument.RootElement.TryGetProperty("hint", out _));
    }
}
