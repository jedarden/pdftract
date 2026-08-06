using MessagePack;
using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents a PDF document with pages and metadata.
/// </summary>
[MessagePackObject]
public record Document
{
    [Key(0)]
    [JsonPropertyName("id")]
    public string Id { get; init; } = string.Empty;

    [Key(1)]
    [JsonPropertyName("pages")]
    public IList<Page> Pages { get; init; } = new List<Page>();

    [Key(2)]
    [JsonPropertyName("metadata")]
    public required Metadata Metadata { get; init; }
}
