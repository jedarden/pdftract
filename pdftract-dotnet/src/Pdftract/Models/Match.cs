using MessagePack;

namespace Pdftract.Models;

/// <summary>
/// Represents a search match result.
/// </summary>
[MessagePackObject]
public record Match
{
    /// <summary>
    /// Page number where the match was found.
    /// </summary>
    [Key(0)]
    public required int PageNumber { get; init; }

    /// <summary>
    /// Matched text content.
    /// </summary>
    [Key(1)]
    public required string Text { get; init; }

    /// <summary>
    /// Surrounding context for the match.
    /// </summary>
    [Key(2)]
    public string? Context { get; init; }
}
