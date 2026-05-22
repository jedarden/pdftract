using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents a single page in the document.
/// </summary>
public record Page
{
    [JsonPropertyName("page")]
    public required int PageIndex { get; init; }

    [JsonPropertyName("width")]
    public required double Width { get; init; }

    [JsonPropertyName("height")]
    public required double Height { get; init; }

    [JsonPropertyName("rotation")]
    public required int Rotation { get; init; }

    [JsonPropertyName("spans")]
    public required List<Span> Spans { get; init; }

    [JsonPropertyName("blocks")]
    public required List<Block> Blocks { get; init; }
}
