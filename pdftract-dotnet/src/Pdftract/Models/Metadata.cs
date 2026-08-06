using MessagePack;

namespace Pdftract.Models;

/// <summary>
/// Represents document metadata.
/// </summary>
[MessagePackObject]
public record Metadata
{
    [Key(0)]
    public string? Title { get; init; }

    [Key(1)]
    public string? Author { get; init; }

    [Key(2)]
    public string? Subject { get; init; }

    [Key(3)]
    public string? Keywords { get; init; }

    [Key(4)]
    public string? Creator { get; init; }

    [Key(5)]
    public string? Producer { get; init; }

    [Key(6)]
    public DateTime? CreatedDate { get; init; }

    [Key(7)]
    public DateTime? ModifiedDate { get; init; }
}
