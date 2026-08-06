using MessagePack;
using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents a PDF document with pages and metadata.
/// </summary>
[GenerateSerializer]
public record Document
{
    [JsonPropertyName("id")]
    public string Id { get; init; } = string.Empty;

    [JsonPropertyName("pages")]
    public IList<Page> Pages { get; init; } = new List<Page>();

    [JsonPropertyName("metadata")]
    public required Metadata Metadata { get; init; }
}
