using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents a search match result.
/// </summary>
public record Match
{
    [JsonPropertyName("text")]
    public required string Text { get; init; }

    [JsonPropertyName("page")]
    public required int Page { get; init; }

    [JsonPropertyName("bbox")]
    public required double[] Bbox { get; init; }

    [JsonPropertyName("context")]
    public required MatchContext Context { get; init; }
}

/// <summary>
/// Provides surrounding text for a match.
/// </summary>
public record MatchContext
{
    [JsonPropertyName("before")]
    public required string Before { get; init; }

    [JsonPropertyName("after")]
    public required string After { get; init; }
}
