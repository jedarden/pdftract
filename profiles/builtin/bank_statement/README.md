# BANK_STATEMENT Profile

Bank statement with account info, period, balances, transaction history

## Match Criteria Summary

A document matches this profile when it displays the characteristic structure of a bank or financial account statement. The classifier identifies statement-specific terminology like "statement of account", "bank statement", "opening balance", "closing balance", and "statement period". Account numbers (often masked with asterisks) and transaction history are key indicators. Structurally, statements are recognized by their monetary columnar layout and the presence of a date column. Statements typically range from 1-10 pages and may include summary sections, account details, and detailed transaction lists with running balances.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| account_number | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| statement_period | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| opening_balance | decimal | Extracted from page text using pattern matching | 123.45 | regex patterns |
| closing_balance | decimal | Extracted from page text using pattern matching | 123.45 | regex patterns |
| transactions | array | Extracted from page text using pattern matching | [...] | table: largest_table_or_central_body |

## Known Limitations

- Multi-account statements (e.g., combined checking/savings) may extract only the primary account
- Credit card statements with payment summaries and purchase categories may not categorize transactions
- Statements with pending vs. posted transaction sections may merge them incorrectly
- Statements in languages other than English may not match due to English-only text patterns
- Very long transaction lists spanning multiple pages may have broken extraction at page boundaries
- Statements with complex formatting (e.g., daily balance graphs, check images) may have reduced extraction quality
- Account number extraction may capture masked numbers (****1234) rather than full account numbers
- Foreign currency statements may extract balances but may not correctly identify currency symbols

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

For statements from specific banks with unique layouts, consider adding bank-specific patterns to improve matching. For credit card statements or investment statements, you may want to create separate profiles with field extractors tailored to those document types.

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
