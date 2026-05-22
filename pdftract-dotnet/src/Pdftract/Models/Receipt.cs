using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents a cryptographic receipt for document verification.
/// </summary>
public record Receipt
{
    [JsonPropertyName("hash")]
    public required string Hash { get; init; }

    [JsonPropertyName("signature")]
    public required string Signature { get; init; }

    [JsonPropertyName("timestamp")]
    public required string Timestamp { get; init; }
}
