using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents a text span with font and position information.
/// </summary>
public record Span
{
    [JsonPropertyName("text")]
    public required string Text { get; init; }

    [JsonPropertyName("bbox")]
    public required double[] Bbox { get; init; }

    [JsonPropertyName("font")]
    public required string Font { get; init; }

    [JsonPropertyName("size")]
    public required double Size { get; init; }

    [JsonPropertyName("confidence")]
    public double? Confidence { get; init; }
}
