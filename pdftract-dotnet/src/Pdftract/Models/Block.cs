using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents a structural block (paragraph, heading, table, etc.).
/// </summary>
public record Block
{
    [JsonPropertyName("kind")]
    public required string Kind { get; init; }

    [JsonPropertyName("text")]
    public required string Text { get; init; }

    [JsonPropertyName("bbox")]
    public required double[] Bbox { get; init; }

    [JsonPropertyName("level")]
    public int? Level { get; init; }
}
