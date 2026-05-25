# Form Profile Fixtures

This directory contains test fixtures for the form document profile.

## Fixture Types

1. **irs_1040.pdf** (2 pages) - IRS Form 1040 U.S. Individual Income Tax Return with standard tax form fields, signature section, and form-based layout
2. **w2.pdf** (1-2 pages) - W-2 Wage and Tax Statement with employee/employer info, wage fields, and tax boxes
3. **i9.pdf** (1-3 pages) - Form I-9 Employment Eligibility Verification with employee attestation section and employer review
4. **expense_report.pdf** (1-2 pages) - Simple expense report with itemized expenses, total calculation, and approval signature
5. **intake_form.pdf** (2-5 pages) - Multi-page new client intake form with personal information, service selection, and consent sections

## Expected Output Format

Each fixture should have a corresponding `*-expected.json` file with the following structure:

```json
{
  "metadata": {
    "document_type": "form",
    "document_type_confidence": 0.XX,
    "document_type_reasons": [...],
    "profile_name": "form",
    "profile_version": "1.0.0",
    "profile_fields": {}
  }
}
```

## Important Notes

The form profile is **degenerate** - it has NO field extractors (`profile_fields: {}`). The form profile:
- Uses `reading_order: line_dominant` for text extraction
- Surfaces `form_fields` from Phase 7.4 (AcroForm field extraction) separately in the extraction output
- Does NOT extract any profile-specific fields

The expected JSON files reflect this degenerate behavior - `profile_fields` is always an empty object `{}`.

## Provenance

All fixtures should be sourced from publicly available form templates or created synthetically with clear provenance documentation. No real forms with PII or confidential information.

## TODO

- [ ] Create irs_1040.pdf and irs_1040-expected.json
- [ ] Create w2.pdf and w2-expected.json
- [ ] Create i9.pdf and i9-expected.json
- [ ] Create expense_report.pdf and expense_report-expected.json
- [ ] Create intake_form.pdf and intake_form-expected.json
