using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents document classification results.
/// </summary>
public record Classification
{
    [JsonPropertyName("category")]
    public required string Category { get; init; }

    [JsonPropertyName("confidence")]
    public required double Confidence { get; init; }

    [JsonPropertyName("tags")]
    public required List<string> Tags { get; init; }

    [JsonPropertyName("heuristics")]
    public required Dictionary<string, bool> Heuristics { get; init; }
}
