# BANK_STATEMENT Profile

Bank statement with account info, period, balances, transactions

## Match Criteria Summary

This profile matches bank statements and account transaction histories. Documents typically contain:

- **Explicit statement markers**: "Statement of account", "Bank statement", "Account statement", "Transaction history"
- **Balance terminology**: "Opening balance", "Closing balance", "Statement period"
- **Account numbers**: Partially masked account numbers (e.g., "****1234" or "Account ****5678")
- **Monetary columnar layout**: Dates, descriptions, and amounts aligned in columns

Bank statements are typically 1-10 pages. The profile expects a tabular transaction layout with date and monetary columns.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| account_number | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| closing_balance | decimal | Extracted from page text using pattern matching | 123.45 | regex patterns |
| opening_balance | decimal | Extracted from page text using pattern matching | 123.45 | regex patterns |
| statement_period | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| transactions | array | Extracted from page text using pattern matching | [...] | table: largest_table_or_central_body |

## Known Limitations

*This section documents known edge cases and failure modes. Contributions to improve extraction quality are welcome.*

- **Multi-page tables**: Only the largest table region is extracted; continuation tables on subsequent pages may be missed
- **Credit card statements**: May match incorrectly if they lack "opening/closing balance" terminology
- **Masked account numbers**: Account number extraction relies on partially masked formats; fully unmasked or non-standard masking may fail
- **International date formats**: Date parsing may fail for non-US formats (DD/MM/YYYY vs MM/DD/YYYY)
- **Running balance columns**: Transactions with running balance columns may extract the balance column instead of the amount column
- **Currency symbols**: Mixed-currency statements (e.g., multi-currency accounts) may extract incorrect amounts

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/profiles/bank_statement/`.

*See the classifier corpus for representative documents.*

## Configuration Tips

To override this profile:

```bash
pdftract profiles export bank_statement > my-profile.yaml
# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-profile.yaml document.pdf
```

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
