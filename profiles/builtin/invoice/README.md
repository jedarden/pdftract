# INVOICE Profile

Commercial invoice with line items, vendor/customer, and totals

## Match Criteria Summary

A document matches this profile when it exhibits the classic structure of a commercial invoice. The classifier looks for explicit invoice terminology such as "invoice", "tax invoice", or "bill to", often paired with vendor/supplier information and customer details. Key indicators include invoice numbers, line item tables (the most reliable structural signal), and payment terms. Page counts typically range from 1-5 pages, with single-page invoices being most common. The presence of line items arranged in tabular format with quantities, unit prices, and amounts is a strong structural signal.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| invoice_number | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| vendor | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| customer | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| invoice_date | date | Extracted from page text using pattern matching | 2024-01-15 | regex patterns |
| due_date | date | Extracted from page text using pattern matching | 2024-01-15 | regex patterns |
| total | decimal | Extracted from page text using pattern matching | 123.45 | regex patterns |
| subtotal | decimal | Extracted from page text using pattern matching | 123.45 | regex patterns |
| tax | decimal | Extracted from page text using pattern matching | 123.45 | regex patterns |
| line_items | array | Extracted from page text using pattern matching | [...] | table: largest_table_or_bottom_half |

## Known Limitations

- Multi-currency invoices may extract the wrong total if currency symbol layout is unusual
- Line items with complex descriptions spanning multiple rows may be truncated or split incorrectly
- Invoices with nested line items (e.g., assemblies with components) may extract only top-level items
- Handwritten invoices or scans with poor OCR quality will have significantly reduced extraction accuracy
- Invoices where vendor/customer information is in logos rather than text may fail to extract those fields
- Credit notes (negative invoices) are not distinguished from regular invoices
- Invoices with multiple tax rates (e.g., different VAT rates) may capture only the aggregated tax total

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/profiles/invoice/`.

*See the classifier corpus for representative documents.*

## Configuration Tips

To override this profile:

```bash
pdftract profiles export invoice > my-profile.yaml
# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-profile.yaml document.pdf
```

For international invoices, you may want to add region-specific text patterns to the `match.text_patterns` list. For invoices with custom fields, add new entries to `profile_fields` with appropriate regex patterns.

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
