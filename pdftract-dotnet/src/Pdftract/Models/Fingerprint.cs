using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents document hash information.
/// </summary>
public record Fingerprint
{
    [JsonPropertyName("hash")]
    public required string Hash { get; init; }

    [JsonPropertyName("page_count")]
    public required int PageCount { get; init; }

    [JsonPropertyName("fast_hash")]
    public required string FastHash { get; init; }

    [JsonPropertyName("metadata")]
    public required Metadata Metadata { get; init; }
}
