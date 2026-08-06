using MessagePack;
using System.Collections.Generic;

namespace Pdftract.Models;

/// <summary>
/// Represents a PDF document with pages and metadata.
/// </summary>
[MessagePackObject]
public record Document
{
    [Key(0)]
    public string Id { get; init; } = string.Empty;

    [Key(1)]
    public IList<Page> Pages { get; init; } = new List<Page>();

    [Key(2)]
    public required Metadata Metadata { get; init; }
}
