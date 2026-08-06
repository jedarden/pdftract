using MessagePack;

namespace Pdftract.Models;

/// <summary>
/// Represents document classification results.
/// </summary>
[MessagePackObject]
public record Classification
{
    /// <summary>
    /// Document category classification.
    /// </summary>
    [Key(0)]
    public required string Category { get; init; }

    /// <summary>
    /// Confidence score for the classification (0.0 to 1.0).
    /// </summary>
    [Key(1)]
    public required double ConfidenceScore { get; init; }
}
