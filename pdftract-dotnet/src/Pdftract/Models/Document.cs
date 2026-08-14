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
    [JsonPropertyName("schema_version")]
    public required string SchemaVersion { get; init; }

    [Key(1)]
    [JsonPropertyName("metadata")]
    public required Metadata Metadata { get; init; }

    [Key(2)]
    [JsonPropertyName("pages")]
    public IList<Page> Pages { get; init; } = new List<Page>();

    [Key(3)]
    [JsonPropertyName("errors")]
    public IList<Error> Errors { get; init; } = new List<Error>();
}
