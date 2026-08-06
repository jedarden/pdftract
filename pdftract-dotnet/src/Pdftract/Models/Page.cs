using MessagePack;
using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents a single page in the document.
/// </summary>
[MessagePackObject]
public record Page
{
    [Key(0)]
    [JsonPropertyName("page")]
    public required int PageIndex { get; init; }

    [Key(1)]
    [JsonPropertyName("width")]
    public double? Width { get; init; }

    [Key(2)]
    [JsonPropertyName("height")]
    public double? Height { get; init; }

    [Key(3)]
    [JsonPropertyName("rotation")]
    public int Rotation { get; init; } = 0;

    [Key(4)]
    [JsonPropertyName("lines")]
    public IList<string> Lines { get; init; } = new List<string>();

    [Key(5)]
    [JsonPropertyName("images")]
    public IList<string> Images { get; init; } = new List<string>();

    [Key(6)]
    [JsonPropertyName("spans")]
    public IList<Span> Spans { get; init; } = new List<Span>();

    [Key(7)]
    [JsonPropertyName("blocks")]
    public IList<Block> Blocks { get; init; } = new List<Block>();
}
