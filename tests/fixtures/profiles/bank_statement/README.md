# Bank Statement Profile Test Fixtures

This directory contains test fixtures for the bank_statement profile extraction.

## Profile Summary

The `bank_statement` profile extracts:
- **account_number**: Account identifier (typically with asterisk notation like *1234)
- **statement_period**: Date range for the statement (e.g., "January 1 - January 31, 2024")
- **opening_balance**: Balance at statement start
- **closing_balance**: Balance at statement end
- **transactions**: Array of transaction records from the main transaction table

## Match Criteria

The profile matches documents that:
- Contain banking terminology ("statement", "transaction", "balance")
- Have at least one table (for transaction listing)
- Contain currency patterns ($X,XXX.XX format)
- Page count between 1 and 10 pages

## Extraction Behavior

- **Reading order**: Line-dominant (bank statements flow left-to-right)
- **Table detection**: Default (capture transaction tables accurately)
- **Readability threshold**: 0.5 (tolerate moderate OCR noise)
- **Headers/footers**: Excluded (page numbers, legal disclaimers filtered out)

## Field Extraction Details

### account_number
- Pattern: Matches "account" followed by asterisk-partial numbers like *1234
- Example: "Account *1234" → "*1234"

### statement_period
- Located near "Statement Period" or "Period" labels
- Returns the full date range string

### opening_balance
- Located near "Opening Balance" or "Beginning Balance"
- Regex captures decimal amounts like $4,250.00
- Parsed as decimal (removes $ and commas)

### closing_balance
- Located near "Closing Balance", "Ending Balance", or "Current Balance"
- Regex captures decimal amounts
- Parsed as decimal

### transactions
- Extracted from the largest table on the page
- Expected columns: date, description, amount, balance (all optional except date and description)
- Falls back to empty array if no table found

## Known Limitations

- Transaction parsing assumes standard tabular layout; unusual formats may fail
- Multi-statement consolidations (multiple accounts) prioritize the largest table
- Negative numbers shown with parentheses or red text are treated as positive values (sign extraction is v2.0+)
- Currency symbols other than $ may require profile updates

## Fixture Coverage

- `checking_account.pdf`: Standard personal checking account (monthly)
- `savings_account.pdf`: Savings account with quarterly statement
- `business_account.pdf`: Business checking with higher transaction volume
- `credit_card_statement.pdf`: Credit card statement with payment/fee structure
- `investment_statement.pdf`: Brokerage statement with dividend/transaction mix
