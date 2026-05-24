# Contract Profile Fixtures

This directory contains test fixtures for the contract document profile.

## Fixture Types

1. **nda.pdf** (1-2 pages) - Non-Disclosure Agreement with two parties, effective date, 1-year term, governing law, and signature blocks
2. **employment.pdf** (5-10 pages) - Employment Agreement with employee/employer parties, start date, at-will term, jurisdiction, and signature blocks
3. **msa.pdf** (20+ pages) - Master Services Agreement with vendor/client parties, effective date, renewal term, governing law section, and signature blocks
4. **service_agreement.pdf** (2-5 pages) - Simple Service Agreement with provider/client parties, effective date, project-based term, governing law, and signatures
5. **real_estate.pdf** (3-10 pages) - Real Estate Purchase Agreement with buyer/seller parties, closing date, contingency period, jurisdiction, and notarized signatures

## Expected Output Format

Each fixture should have a corresponding `expected-output.json` file with the following structure:

```json
{
  "metadata": {
    "document_type": "contract",
    "document_type_confidence": 0.XX,
    "document_type_reasons": [...],
    "profile_name": "contract",
    "profile_version": "1.0.0",
    "profile_fields": {
      "parties": ["Party One", "Party Two"],
      "effective_date": "YYYY-MM-DD",
      "term": "X years" or "until YYYY-MM-DD",
      "governing_law": "State or Jurisdiction",
      "signatures": ["Party One", "Party Two"]
    }
  }
}
```

## Provenance

All fixtures should be sourced from publicly available template contracts or created synthetically with clear provenance documentation. No real contracts with PII or confidential information.

## TODO

- [ ] Create nda.pdf and nda-expected.json
- [ ] Create employment.pdf and employment-expected.json
- [ ] Create msa.pdf and msa-expected.json
- [ ] Create service_agreement.pdf and service_agreement-expected.json
- [ ] Create real_estate.pdf and real_estate-expected.json
