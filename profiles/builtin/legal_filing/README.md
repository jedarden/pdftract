# LEGAL_FILING Profile

Court filing with case number, court, parties, filing date, docket

## Match Criteria Summary

This profile matches court filings and legal documents. Documents typically contain:

- **Case/docket identifiers**: "Case #:", "Docket #:", "Civil Action No."
- **Court naming**: "Court of", "Superior Court", "District Court", "United States District Court"
- **Party designations**: "Plaintiff:", "Defendant:", "Petitioner:", "Respondent:" or "v." notation
- **Court header formatting**: Formal court headers at the top of pages with page numbers

Court filings range from 1-100 pages. The profile expects formal legal formatting with case captions and party identification.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| case_number | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| court | string | Extracted from page text using pattern matching | "example value" | regex patterns, region: first_page_top |
| docket_entries | array | Extracted from page text using pattern matching | [...] | regex patterns, region: after_docket_heading |
| filing_date | date | Extracted from page text using pattern matching | 2024-01-15 | regex patterns |
| parties | array | Extracted from page text using pattern matching | [...] | regex patterns |

## Known Limitations

*This section documents known edge cases and failure modes. Contributions to improve extraction quality are welcome.*

- **Multi-party cases**: Only captures the first two parties (plaintiff/petitioner and defendant/respondent); additional parties are not extracted
- **Cross-claims and counterclaims**: Treated as separate parties; complex multi-party litigation may not extract all parties correctly
- **Sealed/redacted filings**: Redacted case numbers or party names may not extract correctly
- **International courts**: Pattern matching is optimized for US court naming conventions; non-US court formats may fail
- **Docket entry parsing**: Only captures bracketed docket entries ([1], [2]); alternative numbering formats may be missed
- **Amended filings**: Amendments are treated as separate documents; cross-references between filings are not resolved

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/profiles/legal_filing/`.

*See the classifier corpus for representative documents.*

## Configuration Tips

To override this profile:

```bash
pdftract profiles export legal_filing > my-profile.yaml
# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-profile.yaml document.pdf
```

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
