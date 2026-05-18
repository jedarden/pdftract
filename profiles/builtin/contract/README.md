# CONTRACT Profile

Legal contract with parties, effective date, term, signatures

## Match Criteria Summary

Documents matching this profile typically contain:

- **Strong text signals**: Phrases like "agreement is made", "contract agreement", "this agreement", "terms and conditions", "memorandum of understanding"
- **Structural signals**: Presence of signature blocks (detected in bottom 20% of pages), multi-page layout (2+ pages)
- **Page count**: Usually 2-50 pages (contracts are substantive documents)
- **Layout patterns**: Title at top, parties section, numbered or lettered sections, signature blocks at end

The classifier looks for legal agreement terminology combined with multi-page structure and signature blocks. Documents with "agreement" language AND signature blocks match with highest confidence.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| parties | array | Contract parties (vendors, clients, etc.) | `["Acme Corp.", "Global Services LLC"]` | "between X and Y" patterns, "party X:" labels |
| effective_date | date | Date agreement takes effect | 2024-01-15 | "effective date" field with date format |
| term | string | Duration of agreement | "24 months" | "term" patterns with duration |
| governing_law | string | Jurisdiction governing contract | "California" | "governing law" field |
| signatures | array | Signatory names | `["John Smith", "Jane Doe"]` | Bottom of page, "signature:" or "signed:" labels |

## Known Limitations

- **Amendments and addendums**: May not extract correctly if structure differs from main agreement
- **Exhibits and schedules**: Attached exhibits may not be processed; only the main agreement body is extracted
- **Multiple signature pages**: Only signature blocks on the final page are extracted
- **Complex party structures**: Contracts with many parties (e.g., multi-party agreements) may miss some parties
- **Non-standard effective dates**: Effective dates conditional on events (e.g., "upon closing") may not be parsed correctly
- **Redlined documents**: Redlined/track-changes PDFs may confuse the extractor
- **Scanned contracts**: Poor OCR quality can lead to missed fields, especially in fine print
- **Non-English contracts**: Contracts in other languages may not match pattern lists
- **Signature variations**: Electronic signatures, signature stamps, or digital signature images may not be detected

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/contract/`.

The corpus includes contract documents with various agreement types and layouts.

## Configuration Tips

To override this profile for custom contract formats:

```bash
pdftract profiles export contract > my-contract.yaml
# Edit my-contract.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-contract.yaml document.pdf
```

Common customizations:
- Add jurisdiction-specific patterns to `governing_law.extraction.patterns`
- For contracts with specific party naming conventions, update `parties.extraction.patterns`
- Adjust `signatures.extraction.region_hint` if signature blocks are not at the bottom

---

*This README documents the built-in `contract` profile. See `docs/research/document-classification-and-zone-labeling.md` for classifier theory.*
