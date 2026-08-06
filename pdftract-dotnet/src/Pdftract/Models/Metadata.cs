using MessagePack;

namespace Pdftract.Models;

/// <summary>
/// Represents document metadata.
/// </summary>
[GenerateSerializer]
public record Metadata
{
    public string? Title { get; init; }
    public string? Author { get; init; }
    public string? Subject { get; init; }
    public string? Keywords { get; init; }
    public string? Creator { get; init; }
    public string? Producer { get; init; }
    public DateTime? CreatedDate { get; init; }
    public DateTime? ModifiedDate { get; init; }
}
