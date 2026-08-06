using MessagePack;
using System.Collections.Generic;

namespace Pdftract.Models;

/// <summary>
/// Represents a single page in the document.
/// </summary>
[MessagePackObject]
public record Page
{
    [Key(0)]
    public required int PageNumber { get; init; }

    [Key(1)]
    public double? Width { get; init; }

    [Key(2)]
    public double? Height { get; init; }

    [Key(3)]
    public IList<string> Lines { get; init; } = new List<string>();

    [Key(4)]
    public IList<string> Images { get; init; } = new List<string>();
}
