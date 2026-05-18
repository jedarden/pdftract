# LEGAL_FILING Profile

Court filing with case number, court, parties, filing date, docket entries

## Match Criteria Summary

A document matches this profile when it exhibits the formal structure of a court filing or legal pleading. The classifier identifies court-specific terminology like "case #", "docket #", "court of", "plaintiff", "defendant", "petitioner", "respondent", and the legal citation format "v." (versus). Structurally, filings are recognized by their court headers (often with court name, case number, and division) and the presence of page numbers. Filings can range from 1-100 pages depending on the document type (motions, briefs, orders, opinions). The combination of case identifiers, party names, and formal legal language distinguishes this profile from general contracts.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| case_number | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| court | string | Extracted from page text using pattern matching | "example value" | region: first_page_top |
| parties | array | Extracted from page text using pattern matching | [...] | regex patterns |
| filing_date | date | Extracted from page text using pattern matching | 2024-01-15 | regex patterns |
| docket_entries | array | Extracted from page text using pattern matching | [...] | region: after_docket_heading |

## Known Limitations

- Multi-case filings (e.g., consolidated cases) may extract only the primary case number
- Court name extraction may not capture division or department information for larger courts
- Party extraction may fail for cases with many parties (e.g., class actions, multi-defendant cases)
- Docket entries extraction works only for documents with structured docket sections; narrative docket descriptions may not be captured
- Non-English filings may not match due to English-only text patterns
- State court-specific formatting (which varies widely) may not be recognized by generic patterns
- Attachments and exhibits referenced in the filing are not extracted
- Filing type distinctions (complaint, motion, order, opinion) are not made; all match this profile

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

For filings from specific courts (e.g., federal district courts, state superior courts), consider adding court-specific patterns to improve matching. For specialized filing types (e.g., bankruptcy petitions, patent filings), you may want to create separate profiles with field extractors tailored to those document types.

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
