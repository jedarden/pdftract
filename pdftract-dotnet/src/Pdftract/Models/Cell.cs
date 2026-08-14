using MessagePack;
using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents a table cell.
/// </summary>
[MessagePackObject]
public record Cell
{
    [Key(0)]
    [JsonPropertyName("bbox")]
    public IList<float> Bbox { get; init; } = new List<float>();

    [Key(1)]
    [JsonPropertyName("text")]
    public required string Text { get; init; }

    [Key(2)]
    [JsonPropertyName("row")]
    public required uint Row { get; init; }

    [Key(3)]
    [JsonPropertyName("col")]
    public required uint Col { get; init; }

    [Key(4)]
    [JsonPropertyName("rowspan")]
    public uint Rowspan { get; init; }

    [Key(5)]
    [JsonPropertyName("colspan")]
    public uint Colspan { get; init; }
}
