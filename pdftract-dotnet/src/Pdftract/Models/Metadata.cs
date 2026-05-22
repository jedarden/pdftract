using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents document metadata.
/// </summary>
public record Metadata
{
    [JsonPropertyName("title")]
    public string? Title { get; init; }

    [JsonPropertyName("author")]
    public string? Author { get; init; }

    [JsonPropertyName("subject")]
    public string? Subject { get; init; }

    [JsonPropertyName("keywords")]
    public List<string>? Keywords { get; init; }

    [JsonPropertyName("creator")]
    public string? Creator { get; init; }

    [JsonPropertyName("producer")]
    public string? Producer { get; init; }

    [JsonPropertyName("created")]
    public string? Created { get; init; }

    [JsonPropertyName("modified")]
    public string? Modified { get; init; }

    [JsonPropertyName("page_count")]
    public required int PageCount { get; init; }

    [JsonPropertyName("is_encrypted")]
    public bool? IsEncrypted { get; init; }

    [JsonPropertyName("is_signed")]
    public bool? IsSigned { get; init; }
}
