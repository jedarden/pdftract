# INVOICE Profile

Commercial invoice with line items, vendor/customer, and totals

## Match Criteria Summary

This profile matches commercial invoices and bills. Documents typically contain:

- **Invoice indicators**: "Invoice", "Bill to", "Invoice #", "Tax Invoice", "Invoice Number"
- **Payment terminology**: "Due date", "Payment terms", "Purchase order", "PO #"
- **Line item tables**: Tabular layout with items, quantities, unit prices, and amounts
- **Multi-page structure**: Most invoices are 1-5 pages

The profile expects standard invoice formatting with vendor/customer information, line items, and financial totals. It works for service invoices, product invoices, and utility bills.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| invoice_number | string | Unique invoice identifier | "INV-2024-001234" | regex patterns |
| vendor | string | Name of the company issuing the invoice | "Acme Supplies Inc." | regex patterns |
| customer | string | Name of the company or person being billed | "Smith Enterprises LLC" | regex patterns |
| invoice_date | date | Date when the invoice was issued | 2024-01-15 | regex patterns |
| due_date | date | Date when payment is due | 2024-02-15 | regex patterns |
| total | decimal | Final amount due | 1250.00 | regex patterns |
| subtotal | decimal | Sum of line items before tax | 1000.00 | regex patterns |
| tax | decimal | Tax amount (may include VAT/GST) | 250.00 | regex patterns |
| line_items | array | Line items with description, quantity, unit_price, amount | [{description: "Office Chair", quantity: 5, unit_price: 200.00, amount: 1000.00}] | table: largest_table_or_bottom_half |

## Known Limitations

- **Multi-currency invoices**: May extract the wrong total if currency symbol layout is unusual or if multiple currencies are present
- **Line item table detection**: Only the largest table or bottom half is analyzed; invoices with multiple tables may miss some line items
- **Complex tax structures**: Invoices with multiple tax rates (e.g., different VAT rates for different items) may only extract the total tax, not the breakdown
- **Handwritten modifications**: Notes or changes written on the invoice are not detected
- **Purchase order matching**: PO numbers are extracted but not validated against external systems
- **Vendor name extraction**: Assumes vendor name appears near "from:", "vendor:", or "supplier:" markers; alternative layouts may miss this field
- **Non-English invoices**: Pattern matching is primarily English-language focused
- **Credit notes**: Treated as invoices; negative amounts may not be handled correctly
- **Discounts and coupons**: Line-item discounts may not be attributed correctly; discounts are often extracted as separate line items

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/invoice/` (50+ representative invoices).

*See the classifier corpus for representative documents.*

## Configuration Tips

To override this profile:

```bash
pdftract profiles export invoice > my-profile.yaml
# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-profile.yaml document.pdf
```

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
