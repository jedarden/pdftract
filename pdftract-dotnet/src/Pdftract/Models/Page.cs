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
    public required int PageNumber { get; init; }

    [Key(1)]
    [JsonPropertyName("width")]
    public double? Width { get; init; }

    [Key(2)]
    [JsonPropertyName("height")]
    public double? Height { get; init; }

    [Key(3)]
    [JsonPropertyName("lines")]
    public IList<string> Lines { get; init; } = new List<string>();

    [Key(4)]
    [JsonPropertyName("images")]
    public IList<string> Images { get; init; } = new List<string>();
}
