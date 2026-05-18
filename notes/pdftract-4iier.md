# pdftract-4iier: Per-profile README Documentation

## Summary

Completed per-profile README documentation for all 9 built-in profiles. Each README follows the consistent 6-section structure specified in the acceptance criteria.

## Files Updated

All 9 README files exist at `profiles/builtin/<type>/README.md`:
1. `profiles/builtin/invoice/README.md` - Invoice profile documentation
2. `profiles/builtin/receipt/README.md` - Receipt profile documentation (fixed date type: string → date)
3. `profiles/builtin/contract/README.md` - Contract profile documentation
4. `profiles/builtin/scientific_paper/README.md` - Scientific paper profile documentation
5. `profiles/builtin/slide_deck/README.md` - Slide deck profile documentation
6. `profiles/builtin/form/README.md` - Form profile documentation (degenerate case: no field extractors)
7. `profiles/builtin/bank_statement/README.md` - Bank statement profile documentation
8. `profiles/builtin/legal_filing/README.md` - Legal filing profile documentation
9. `profiles/builtin/book_chapter/README.md` - Book chapter profile documentation

## xtask Implementation

The `xtask/src/main.rs` already contains the `doc-profile` and `doc-profiles` commands that generate README skeletons from profile YAML files. This was already implemented and working.

## Bug Fix

Fixed receipt README: changed `date` field type from `string` to `date` to match the YAML definition (receipt/profile.yaml has `type: date`).

## Acceptance Criteria Status

- ✅ All nine README files exist at the documented paths
- ✅ Each follows the consistent 6-section structure (Title/Description, Match Criteria Summary, Extracted Fields, Known Limitations, Sample Input, Configuration Tips)
- ✅ Extracted Fields tables match the corresponding profile YAML's profile_fields
- ✅ Known Limitations is non-empty and document-specific for all profiles
- ✅ Sample Input Pointer links to actual fixtures in tests/fixtures/classifier/
- ✅ xtask doc-profile skeleton generator scripted (already implemented)

## Fixture Path Verification

All Sample Input sections reference actual fixture files:
- invoice: `tests/fixtures/classifier/invoice/` (50+ files)
- receipt: `tests/fixtures/classifier/misc/` (samples 01-08.pdf)
- contract: `tests/fixtures/classifier/contract/` (50+ files)
- scientific_paper: `tests/fixtures/classifier/scientific_paper/` (50+ files)
- slide_deck: `tests/fixtures/classifier/misc/` (samples 24-30.pdf)
- form: `tests/fixtures/classifier/misc/` (samples 09-16.pdf)
- bank_statement: `tests/fixtures/classifier/misc/` (samples 17-23.pdf)
- legal_filing: `tests/fixtures/classifier/misc/` (samples 31-37.pdf)
- book_chapter: `tests/fixtures/classifier/misc/` (samples 38-43.pdf)

## Testing

Verified xtask compiles and runs:
```bash
cd xtask && cargo build  # Success
./target/debug/xtask     # Shows doc-profile and doc-profiles commands
```

## PASS Items

All acceptance criteria PASS:
- All 9 README files exist at correct paths
- All follow consistent 6-section structure
- All Extracted Fields tables match YAML profile_fields
- All Known Limitations sections are non-empty and profile-specific
- All Sample Input pointers reference existing fixtures
- xtask doc-profile skeleton generator is implemented

## WARN Items

None. All criteria met without warnings.
