# INVOICE Profile

Commercial invoice with line items, vendor/customer, and totals

## Match Criteria Summary

Documents matching this profile typically contain:

- **Strong text signals**: Words like "invoice", "bill to", "invoice #", "tax invoice", "due date", "purchase order"
- **Structural signals**: Presence of a line item table (detected as the largest table or in the bottom half of the first page)
- **Page count**: Usually 1-5 pages (invoices are rarely longer)
- **Layout patterns**: Vendor information at top, billing details, line items table, and totals at bottom

The classifier looks for invoice-specific terminology combined with tabular data structures. Documents with both "invoice" terminology AND monetary tables match with highest confidence.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| invoice_number | string | Unique invoice identifier | "INV-2024-0154" | Regex patterns: `invoice\s*[#:]?\s*([A-Z0-9-]+)` |
| vendor | string | Company issuing the invoice | "Acme Supplies Inc." | Regex patterns: vendor/supplier/company fields |
| customer | string | Company billed to | "Global Tech Corp." | Regex patterns: "bill to" section |
| invoice_date | date | Date invoice was issued | 2024-01-15 | Regex patterns: "invoice date" field |
| due_date | date | Payment deadline | 2024-02-14 | Regex patterns: "due date" or "payment due" fields |
| total | decimal | Total amount due | 1250.00 | Regex patterns: "total" or "amount due" fields |
| subtotal | decimal | Amount before tax | 1000.00 | Regex patterns: "subtotal" field |
| tax | decimal | Tax amount | 250.00 | Regex patterns: "tax", "vat", "gst" fields |
| line_items | array | Array of line item objects | `[{description: "Widget", quantity: 10, unit_price: 100.00, amount: 1000.00}]` | Table extraction from largest table |

## Known Limitations

- **Multi-currency invoices**: May extract the wrong total if currency symbols appear in multiple places; the profile matches the first currency symbol near "total"
- **Complex line items**: Line items spanning multiple rows (e.g., multi-line descriptions) may be split incorrectly; table extraction assumes single-row items
- **Handwritten or scanned invoices**: OCR errors can cause missed fields; the profile relies on clean text extraction
- **Non-standard layouts**: Invoices with line items on multiple pages may only extract items from the first page
- **Multiple invoices in one PDF**: Only the first invoice-like structure is extracted
- **Discount handling**: Discounts are not explicitly extracted; they may appear as negative line items or be missed entirely
- **Invoice variations**: Non-English invoices (e.g., "factura", "rechnung") may not match if the pattern list isn't localized

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/invoice/`.

The corpus includes 50 invoice documents covering various formats and layouts.

## Configuration Tips

To override this profile for custom invoice formats:

```bash
pdftract profiles export invoice > my-invoice.yaml
# Edit my-invoice.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-invoice.yaml document.pdf
```

Common customizations:
- Add company-specific invoice number patterns to `invoice_number.extraction.patterns`
- Adjust `line_items.extraction.table_region` if invoices use non-standard table placement
- Add localized patterns for non-English invoices

---

*This README documents the built-in `invoice` profile. See `docs/research/document-classification-and-zone-labeling.md` for classifier theory.*
