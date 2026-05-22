using System.Text.Json.Serialization;
using System.Text.Json;

namespace Pdftract.Models;

/// <summary>
/// Source-generated JSON serialization context for all pdftract model types.
/// This enables Native AOT compilation by avoiding reflection-based serialization.
/// </summary>
[JsonSourceGenerationOptions(
    PropertyNamingPolicy = JsonKnownNamingPolicy.SnakeCaseLower,
    WriteIndented = false,
    DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull)]
[JsonSerializable(typeof(Document))]
[JsonSerializable(typeof(Page))]
[JsonSerializable(typeof(Span))]
[JsonSerializable(typeof(Block))]
[JsonSerializable(typeof(Metadata))]
[JsonSerializable(typeof(Match))]
[JsonSerializable(typeof(MatchContext))]
[JsonSerializable(typeof(Fingerprint))]
[JsonSerializable(typeof(Classification))]
[JsonSerializable(typeof(Receipt))]
[JsonSerializable(typeof(ReceiptInfo))]
public partial class PdftractJsonContext : JsonSerializerContext;
