using MessagePack;
using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents a single page in the document.
/// </summary>
[GenerateSerializer]
public record Page
{
    [JsonPropertyName("page")]
    public required int PageNumber { get; init; }

    [JsonPropertyName("width")]
    public double? Width { get; init; }

    [JsonPropertyName("height")]
    public double? Height { get; init; }

    [JsonPropertyName("lines")]
    public IList<string> Lines { get; init; } = new List<string>();

    [JsonPropertyName("images")]
    public IList<string> Images { get; init; } = new List<string>();
}
