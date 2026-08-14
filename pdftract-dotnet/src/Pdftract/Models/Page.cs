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
    [JsonPropertyName("page_index")]
    public required uint PageIndex { get; init; }

    [Key(1)]
    [JsonPropertyName("page_number")]
    public required uint PageNumber { get; init; }

    [Key(2)]
    [JsonPropertyName("page_label")]
    public string? PageLabel { get; init; }

    [Key(3)]
    [JsonPropertyName("width")]
    public required float Width { get; init; }

    [Key(4)]
    [JsonPropertyName("height")]
    public required float Height { get; init; }

    [Key(5)]
    [JsonPropertyName("rotation")]
    public required uint Rotation { get; init; }

    [Key(6)]
    [JsonPropertyName("type")]
    public required string PageType { get; init; }

    [Key(7)]
    [JsonPropertyName("spans")]
    public IList<Span> Spans { get; init; } = new List<Span>();

    [Key(8)]
    [JsonPropertyName("blocks")]
    public IList<Block> Blocks { get; init; } = new List<Block>();

    [Key(9)]
    [JsonPropertyName("tables")]
    public IList<Table> Tables { get; init; } = new List<Table>();

    [Key(10)]
    [JsonPropertyName("annotations")]
    public IList<Annotation> Annotations { get; init; } = new List<Annotation>();
}
