# LEGAL_FILING Profile

Court filing with case number, court, parties, filing date, docket

## Match Criteria Summary

Documents matching this profile typically contain:

- **Strong text signals**: Words like "case #:", "docket #:", "court of", "superior court", "district court"
- **Structural signals**: Court header at top, page numbers, signature blocks
- **Page count**: Usually 1-100 pages (filings vary by document type)
- **Layout patterns**: Court caption at top (court name, case number, parties), document body, docket entries or certificate of service

The classifier looks for legal filing terminology combined with court header structures. Documents with "case/docket" terminology AND court headers match with highest confidence.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| case_number | string | Case or docket number | "CV-2024-001234" or "1:24-cv-00123" | "case", "docket", or "civil action no." fields |
| court | string | Court name | "Superior Court of California" | First page top, court name patterns |
| parties | array | Parties to the case | `["Smith", "Jones"]` | "plaintiff", "defendant", "petitioner", "respondent", or "v." patterns |
| filing_date | date | Date document was filed | 2024-01-15 | "filed", "submitted", or "date filed" fields |
| docket_entries | array | Docket or proceeding entries | `["[1] Complaint filed"]` | After "docket" heading, numbered list |

## Known Limitations

- **Multi-case filings**: Filings referencing multiple cases may only extract the first case number
- **Sealed filings**: Redacted or sealed filings may have missing information
- **Exhibit attachments**: Exhibits attached to filings are not processed separately
- **Complex caption formats**: Some courts use non-standard caption formats that may not parse correctly
- **Non-English filings**: Filings in languages other than English may not match pattern lists
- **Scanned filings**: Poor OCR quality can lead to missed fields, especially in dense captions
- **Multiple parties**: Cases with many parties (e.g., class actions) may not extract all parties
- **Electronically filed documents**: Some e-filing systems add headers/footers that may interfere with extraction
- **State-specific formats**: Different states have different caption formats; some may not be supported

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/contract/`.

Legal filing fixtures are typically multi-page documents with court captions at the top.

## Configuration Tips

To override this profile for custom legal filing formats:

```bash
pdftract profiles export legal_filing > my-filing.yaml
# Edit my-filing.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-filing.yaml document.pdf
```

Common customizations:
- Add court-specific patterns to `case_number.extraction.patterns`
- For state-specific formats, update `court.extraction.patterns` with local court names
- Adjust `parties.extraction.patterns` for different party types (e.g., "appellant", "appellee")

---

*This README documents the built-in `legal_filing` profile. See `docs/research/document-classification-and-zone-labeling.md` for classifier theory.*
