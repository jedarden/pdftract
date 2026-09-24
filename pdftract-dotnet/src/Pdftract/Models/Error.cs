using MessagePack;
using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Canonical structured extraction diagnostic.
///
/// Full JSON errors and the NDJSON footer use this model. The compact Rust
/// metadata API retains its legacy string diagnostics array separately.
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

    /// <summary>Optional zero-based page index; omitted when not applicable.</summary>
    [Key(3)]
    [JsonPropertyName("page_index")]
    public int? PageIndex { get; init; }

    /// <summary>Optional PDF indirect-object location.</summary>
    [Key(4)]
    [JsonPropertyName("location")]
    public ObjectLocation? Location { get; init; }

    /// <summary>Optional catalog hint; omitted when not available.</summary>
    [Key(5)]
    [JsonPropertyName("hint")]
    public string? Hint { get; init; }
}

/// <summary>PDF indirect-object location attached to a diagnostic.</summary>
[MessagePackObject]
public record ObjectLocation
{
    [Key(0)]
    [JsonPropertyName("object_number")]
    public required int ObjectNumber { get; init; }

    [Key(1)]
    [JsonPropertyName("generation_number")]
    public required int GenerationNumber { get; init; }
}
