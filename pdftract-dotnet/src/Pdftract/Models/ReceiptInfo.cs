using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Receipt verification information.
/// </summary>
public record ReceiptInfo
{
    /// <summary>
    /// Whether the receipt is valid.
    /// </summary>
    [JsonPropertyName("valid")]
    public required bool Valid { get; init; }

    /// <summary>
    /// Merchant name.
    /// </summary>
    [JsonPropertyName("merchant")]
    public string? Merchant { get; init; }

    /// <summary>
    /// Transaction amount.
    /// </summary>
    [JsonPropertyName("amount")]
    public double? Amount { get; init; }

    /// <summary>
    /// Transaction date.
    /// </summary>
    [JsonPropertyName("date")]
    public string? Date { get; init; }

    /// <summary>
    /// Additional receipt details.
    /// </summary>
    [JsonPropertyName("details")]
    public Dictionary<string, object>? Details { get; init; }
}
