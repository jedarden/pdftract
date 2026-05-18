# pdftract-4iier: Profile README Documentation

## Summary

Created per-profile README documentation for all 9 built-in profiles.

## Files Created

### Profile YAML Files (9)
- `profiles/builtin/invoice/profile.yaml` - Invoice with line items, vendor/customer, totals
- `profiles/builtin/receipt/profile.yaml` - POS receipt with items, payment method
- `profiles/builtin/contract/profile.yaml` - Legal contract with parties, effective date, term, signatures
- `profiles/builtin/scientific_paper/profile.yaml` - Academic paper with title, authors, abstract, DOI, references
- `profiles/builtin/slide_deck/profile.yaml` - Presentation slides with title, presenter, date, slide titles
- `profiles/builtin/form/profile.yaml` - Fillable form (degenerate case: no field extractor, uses Phase 7.4 form_fields)
- `profiles/builtin/bank_statement/profile.yaml` - Bank statement with account info, period, balances, transactions
- `profiles/builtin/legal_filing/profile.yaml` - Court filing with case number, court, parties, filing date, docket
- `profiles/builtin/book_chapter/profile.yaml` - Book chapter with title, chapter number, author, section headings

### Profile README Files (9)
Each README follows the consistent 6-section structure:
1. Title and one-line description
2. Match Criteria Summary - prose description of matching signals
3. Extracted Fields - table with field_name, type, description, example_value, source_location_hint
4. Known Limitations - document-specific edge cases and failure modes
5. Sample Input - pointer to fixtures
6. Configuration Tips - how to override via `--profile` or export/edit

### xtask Skeleton Generator
- `xtask/Cargo.toml` - Cargo manifest for xtask binary
- `xtask/src/main.rs` - Rust code for `xtask doc-profile <name>` and `xtask doc-profiles` commands

## Acceptance Criteria Status

- ✅ All nine README files exist at the documented paths
- ✅ Each follows the consistent 6-section structure
- ✅ Extracted Fields tables match the corresponding profile YAML's profile_fields
- ✅ Known Limitations is non-empty and document-specific for each profile
- ✅ Sample Input Pointer links to actual fixtures in tests/fixtures/classifier/ or tests/fixtures/profiles/
- ✅ xtask doc-profile skeleton generator scripted (Rust code in xtask/)

## Notes

- The form profile README correctly documents that it's a degenerate case (no field extractor, uses Phase 7.4 form_fields)
- The slide_deck README notes that extraction quality depends heavily on the PDF exporter
- Each profile's Known Limitations section is comprehensive and specific to that document type
- All READMEs reference docs/research/document-classification-and-zone-labeling.md for classifier theory
- The xtask generator is a starting point; it would need workspace integration to build/run
