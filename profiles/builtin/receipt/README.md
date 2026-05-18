# RECEIPT Profile

Point-of-sale or purchase receipt with items, payment method

## Match Criteria Summary

A document matches this profile when it displays the typical characteristics of a point-of-sale receipt. The classifier identifies receipt-specific terminology like "store receipt", "total sold", "change due", and payment method indicators. Structurally, receipts are recognized by their narrow aspect ratio (often mimicking thermal printer paper), columnar layout with monetary values, and compact single-page format. The presence of monetary columns aligned to the right side of the document is a strong structural signal. Receipts are almost always single-page documents with a vertical orientation.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| merchant | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| date | date | Extracted from page text using pattern matching | 2024-01-15 | regex patterns |
| total | decimal | Extracted from page text using pattern matching | 123.45 | regex patterns |
| tax | decimal | Extracted from page text using pattern matching | 123.45 | regex patterns |
| items | array | Extracted from page text using pattern matching | [...] | columns: monetary_columns |
| payment_method | string | Extracted from page text using pattern matching | "example value" | regex patterns |

## Known Limitations

- Very long receipts (e.g., from home improvement stores) may fold across multiple scan pages, breaking extraction
- Receipts with faint thermal print or low-resolution scans may have poor OCR quality
- Handwritten receipts (e.g., from contractors) may not match the profile due to lack of columnar structure
- Receipts in right-to-left languages (Arabic, Hebrew) may fail monetary column detection
- Multi-store returns or exchange receipts with complex itemization may extract items incorrectly
- Receipts with multiple transactions on one document (e.g., daily register tape) are not handled
- Tip lines on restaurant receipts may be confused with subtotal/total fields

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/profiles/receipt/`.

*See the classifier corpus for representative documents.*

## Configuration Tips

To override this profile:

```bash
pdftract profiles export receipt > my-profile.yaml
# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-profile.yaml document.pdf
```

For receipts from specific merchants with custom layouts, consider adding merchant-specific patterns to the `match.text_patterns` list. For receipts with unique item formats, customize the `items` field's extraction schema.

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
