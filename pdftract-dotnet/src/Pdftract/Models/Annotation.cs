using MessagePack;
using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents a page annotation (highlight, note, link, etc.).
/// </summary>
[MessagePackObject]
public record Annotation
{
    [Key(0)]
    [JsonPropertyName("type")]
    public required string Type { get; init; }

    [Key(1)]
    [JsonPropertyName("bbox")]
    public IList<float> Bbox { get; init; } = new List<float>();

    [Key(2)]
    [JsonPropertyName("content")]
    public string? Content { get; init; }
}
