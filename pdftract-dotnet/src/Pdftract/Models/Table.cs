using MessagePack;
using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents a table structure.
/// </summary>
[MessagePackObject]
public record Table
{
    [Key(0)]
    [JsonPropertyName("bbox")]
    public IList<float> Bbox { get; init; } = new List<float>();

    [Key(1)]
    [JsonPropertyName("rows")]
    public uint Rows { get; init; }

    [Key(2)]
    [JsonPropertyName("columns")]
    public uint Columns { get; init; }

    [Key(3)]
    [JsonPropertyName("cells")]
    public IList<Cell> Cells { get; init; } = new List<Cell>();
}
