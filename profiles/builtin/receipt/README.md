# RECEIPT Profile

Point-of-sale or purchase receipt with items, payment method

## Match Criteria Summary

Documents matching this profile typically contain:

- **Strong text signals**: Words like "receipt", "store receipt", "register receipt", "transaction receipt"
- **Structural signals**: Monetary columnar layout (items with prices aligned), narrow or square page aspect ratio (typical of thermal receipt paper)
- **Page count**: Usually 1 page (receipts are single-page documents)
- **Layout patterns**: Merchant name at top, item list with prices in columns, total at bottom, payment method near bottom

The classifier looks for receipt-specific terminology combined with narrow-aspect-ratio pages and columnar monetary data. Thermal receipts (narrow width) are strong indicators.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| merchant | string | Store or business name | "Whole Foods Market" | First line or "store/merchant" field |
| date | type: date | Transaction date | 2024-01-15 | Date field near top or middle |
| total | decimal | Total amount paid | 87.43 | "total" field near bottom |
| tax | decimal | Tax amount | 6.32 | "tax" field in item list or near total |
| items | array | Array of purchased items | `[{name: "Organic Apples", quantity: 1.5, price: 2.99}]` | Columnar extraction from monetary columns |
| payment_method | string | How payment was made | "Visa" | Keywords: cash, credit, debit, card type |

## Known Limitations

- **Long receipts**: Very long receipts (e.g., pharmacy receipts with many items) may have extraction errors in the middle section
- **Multi-page receipts**: Rare but possible; currently only processes first page
- **Thermal printer fading**: Faded thermal receipts may have OCR errors leading to missed items
- **Handwritten receipts**: Items added by hand may not be extracted
- **Non-itemized receipts**: Some receipts show only the total (e.g., fast food); item array will be empty
- **Coupons and discounts**: Discounts may appear as negative items or be missed entirely
- **Non-standard layouts**: Receipts with non-columnar layouts (e.g., handwritten, formatted invoices) may not extract items correctly
- **Non-ASCII characters**: Receipts with non-Latin scripts may have encoding issues
- **Receipts with multiple transactions**: Combined receipts (e.g., return + purchase) may confuse the extractor

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/misc/`.

Receipt fixtures are typically single-page narrow documents with itemized lists.

## Configuration Tips

To override this profile for custom receipt formats:

```bash
pdftract profiles export receipt > my-receipt.yaml
# Edit my-receipt.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-receipt.yaml document.pdf
```

Common customizations:
- Add store-specific item patterns to `items.extraction.schema`
- Adjust `payment_method.extraction.patterns` for additional payment types (e.g., "Apple Pay", "Samsung Pay")
- For receipts with multiple transaction types, consider creating separate profiles

---

*This README documents the built-in `receipt` profile. See `docs/research/document-classification-and-zone-labeling.md` for classifier theory.*
