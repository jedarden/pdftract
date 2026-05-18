# BANK_STATEMENT Profile

Bank statement with account info, period, balances, transactions

## Match Criteria Summary

Documents matching this profile typically contain:

- **Strong text signals**: Words like "statement of account", "bank statement", "account statement", "transaction history"
- **Structural signals**: Monetary columnar layout, date columns, tabular transaction data
- **Page count**: Usually 1-10 pages (monthly statements vary by account activity)
- **Layout patterns**: Account info at top, statement period, opening balance, transaction list table, closing balance

The classifier looks for bank statement terminology combined with monetary tabular data. Documents with "statement" terminology AND date/amount columns match with highest confidence.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| account_number | string | Account identifier | "****1234" or "8374921" | "account" or "acct" field, often masked |
| statement_period | string | Date range covered | "January 1 - January 31, 2024" | "statement period" field |
| opening_balance | decimal | Balance at period start | 1500.00 | "opening balance" or "beginning balance" |
| closing_balance | decimal | Balance at period end | 1750.00 | "closing balance", "ending balance", or "current balance" |
| transactions | array | Transaction records | `[{date: "2024-01-15", description: "GROCERY STORE", amount: -85.32, balance: 1664.68}]` | Largest table or central body |

## Known Limitations

- **Multi-currency accounts**: Transactions in multiple currencies may extract incorrectly
- **Continued statements**: Statements spanning multiple PDF files (e.g., "continued on next page") may not link correctly
- **Scanned statements**: Poor OCR quality can lead to transaction parsing errors, especially with dense tables
- **Non-standard layouts**: Statements with unusual layouts (e.g., credit card statements with tiered rates) may not extract correctly
- **Pending transactions**: Pending or holds transactions may be excluded or formatted differently
- **Transaction descriptions**: Very long descriptions may be truncated or wrapped incorrectly
- **Multiple accounts**: Statements covering multiple accounts (e.g., combined statements) may not separate accounts correctly
- **Non-English statements**: Statements in other languages may not match pattern lists
- **Check images**: Statements with embedded check images may have OCR artifacts

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/misc/`.

Bank statement fixtures are typically multi-page documents with transaction tables.

## Configuration Tips

To override this profile for custom bank statement formats:

```bash
pdftract profiles export bank_statement > my-statement.yaml
# Edit my-statement.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-statement.yaml document.pdf
```

Common customizations:
- Add bank-specific account number patterns to `account_number.extraction.patterns`
- Adjust `transactions.extraction.table_region` for non-standard table placement
- For credit card statements, add fields like `minimum_payment`, `payment_due_date`

---

*This README documents the built-in `bank_statement` profile. See `docs/research/document-classification-and-zone-labeling.md` for classifier theory.*
