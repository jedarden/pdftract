using MessagePack;
using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents an extraction error/diagnostic.
/// </summary>
[MessagePackObject]
public record Error
{
    [Key(0)]
    [JsonPropertyName("code")]
    public required string Code { get; init; }

    [Key(1)]
    [JsonPropertyName("message")]
    public required string Message { get; init; }

    [Key(2)]
    [JsonPropertyName("severity")]
    public required string Severity { get; init; }
}
