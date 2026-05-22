using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents a PDF document with pages and metadata.
/// </summary>
[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.SnakeCaseLower)]
[JsonSerializable(typeof(Document))]
public partial class DocumentContext : JsonSerializerContext;

public record Document
{
    [JsonPropertyName("schema_version")]
    public string SchemaVersion { get; init; } = string.Empty;

    [JsonPropertyName("pages")]
    public required List<Page> Pages { get; init; }

    [JsonPropertyName("metadata")]
    public required Metadata Metadata { get; init; }
}
