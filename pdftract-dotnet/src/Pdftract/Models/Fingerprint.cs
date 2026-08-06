using MessagePack;

namespace Pdftract.Models;

/// <summary>
/// Represents a document fingerprint for identification.
/// </summary>
[MessagePackObject]
public record Fingerprint
{
    /// <summary>
    /// Document hash value.
    /// </summary>
    [Key(0)]
    public required string Hash { get; init; }

    /// <summary>
    /// Document size in bytes.
    /// </summary>
    [Key(1)]
    public required long Size { get; init; }

    /// <summary>
    /// Number of pages in the document.
    /// </summary>
    [Key(2)]
    public required int PageCount { get; init; }
}
