using MessagePack;
using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents document metadata.
/// </summary>
[MessagePackObject]
public record Metadata
{
    [Key(0)]
    [JsonPropertyName("title")]
    public string? Title { get; init; }

    [Key(1)]
    [JsonPropertyName("author")]
    public string? Author { get; init; }

    [Key(2)]
    [JsonPropertyName("subject")]
    public string? Subject { get; init; }

    [Key(3)]
    [JsonPropertyName("keywords")]
    public string? Keywords { get; init; }

    [Key(4)]
    [JsonPropertyName("creator")]
    public string? Creator { get; init; }

    [Key(5)]
    [JsonPropertyName("producer")]
    public string? Producer { get; init; }

    [Key(6)]
    [JsonPropertyName("created_date")]
    public DateTime? CreatedDate { get; init; }

    [Key(7)]
    [JsonPropertyName("modified_date")]
    public DateTime? ModifiedDate { get; init; }
}
