using MessagePack;
using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents PDF document metadata (title, author, dates, etc.).
/// </summary>
[GenerateSerializer]
public record Metadata
{
    [JsonPropertyName("title")]
    public string? Title { get; init; }

    [JsonPropertyName("author")]
    public string? Author { get; init; }

    [JsonPropertyName("subject")]
    public string? Subject { get; init; }

    [JsonPropertyName("keywords")]
    public string? Keywords { get; init; }

    [JsonPropertyName("creator")]
    public string? Creator { get; init; }

    [JsonPropertyName("producer")]
    public string? Producer { get; init; }

    [JsonPropertyName("created_date")]
    public DateTime? CreatedDate { get; init; }

    [JsonPropertyName("modified_date")]
    public DateTime? ModifiedDate { get; init; }
}
