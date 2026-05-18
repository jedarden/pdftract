# RECEIPT Profile

Point-of-sale or purchase receipt with items, payment method

## Match Criteria Summary

This profile matches point-of-sale and purchase receipts. Documents typically contain:

- **Receipt indicators**: "receipt", "store receipt", "register receipt", "transaction receipt"
- **Transaction language**: "total sold", "change due", "cash/credit", "card payment"
- **Columnar monetary layout**: Multiple columns with numeric values aligned (typical POS layout)
- **Narrow or square aspect ratio**: Most receipts are narrow thermal printouts

Most receipts are single-page. The profile expects dense text with itemized lists and payment totals.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| merchant | string | Name of the store or vendor | "COFFEE HOUSE" | regex patterns |
| date | date | Transaction date | 2024-01-15 | regex patterns |
| total | decimal | Final transaction amount | 15.47 | regex patterns |
| tax | decimal | Tax amount charged | 1.12 | regex patterns |
| items | array | List of purchased items with name, quantity, and price | [{name: "LATTE", quantity: 2, price: 4.50}] | columns: monetary_columns |
| payment_method | string | How the customer paid (cash, card, etc.) | "VISA" | regex patterns |

## Known Limitations

- **Thermal printer fade**: Faded or low-contrast thermal printouts may have missing text
- **Multi-page receipts**: Uncommon, but some retailers print multiple pages; only the first page is analyzed
- **Non-English receipts**: Pattern matching is primarily English-language focused
- **Handwritten modifications**: Tips or adjustments written on the receipt are not detected
- **Complex discounts**: Line-item discounts or coupons may not be attributed correctly
- **Barcode-heavy layouts**: Some receipts have large barcode areas that interfere with text extraction
- **Very narrow receipts**: Extremely narrow thermal printouts (< 2 inches) may have character recognition issues

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/misc/` (receipt samples: 01-08.pdf).

*See the classifier corpus for representative documents.*

## Configuration Tips

To override this profile:

```bash
pdftract profiles export receipt > my-profile.yaml
# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-profile.yaml document.pdf
```

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
