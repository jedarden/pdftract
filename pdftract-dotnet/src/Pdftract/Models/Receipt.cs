using MessagePack;

namespace Pdftract.Models;

/// <summary>
/// Represents a transaction receipt extracted from a document.
/// </summary>
[MessagePackObject]
public record Receipt
{
    /// <summary>
    /// Seller or merchant name.
    /// </summary>
    [Key(0)]
    public string? Seller { get; init; }

    /// <summary>
    /// Transaction date.
    /// </summary>
    [Key(1)]
    public DateTime? Date { get; init; }

    /// <summary>
    /// Total transaction amount.
    /// </summary>
    [Key(2)]
    public decimal? Total { get; init; }

    /// <summary>
    /// List of line items in the receipt.
    /// </summary>
    [Key(3)]
    public IList<ReceiptLineItem> LineItems { get; init; } = Array.Empty<ReceiptLineItem>();
}

/// <summary>
/// Represents a single line item on a receipt.
/// </summary>
[MessagePackObject]
public record ReceiptLineItem
{
    /// <summary>
    /// Item description or name.
    /// </summary>
    [Key(0)]
    public required string Description { get; init; }

    /// <summary>
    /// Quantity of items.
    /// </summary>
    [Key(1)]
    public decimal? Quantity { get; init; }

    /// <summary>
    /// Price per unit.
    /// </summary>
    [Key(2)]
    public decimal? UnitPrice { get; init; }

    /// <summary>
    /// Total price for this line item.
    /// </summary>
    [Key(3)]
    public decimal? Total { get; init; }
}
