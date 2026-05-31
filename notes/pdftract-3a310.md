# Phase 7.10 Coordinator Verification Note

**Bead ID:** pdftract-3a310
**Date:** 2026-05-31
**Commit:** 80dbf0f (feat(profiles): add profile infrastructure and initial fixtures)

## Status: CANNOT CLOSE - Dependent epic incomplete

The coordinator `pdftract-3a310` cannot be closed because its dependent epic `pdftract-1lp2` (Profile Authoring) is still **open**.

## Dependency Chain

```
pdftract-3a310 (Phase 7.10 coordinator)
├── pdftract-3zhf (Phase 7.2 coordinator) - CLOSED ✓
├── pdftract-2mw6 (Phase 7.4 coordinator) - CLOSED ✓
└── pdftract-1lp2 (Profile Authoring epic) - OPEN ✗
```

## What Was Completed (This Session)

### Profile Infrastructure Code
Committed in 80dbf0f:
- `crates/pdftract-core/src/profiles/apply_profile.rs` - Profile application logic
- `crates/pdftract-core/src/profiles/extraction.rs` - Extraction override handling
- `crates/pdftract-core/src/profiles/extraction_loader.rs` - Extraction option deserialization
- `crates/pdftract-core/src/profiles/field_extractor.rs` - Field DSL evaluator
- `crates/pdftract-core/src/profiles/match_eval.rs` - Match DSL evaluator
- `crates/pdftract-cli/src/profiles_cmd.rs` - profiles subcommand implementation
- Updated `crates/pdftract-core/src/profiles/mod.rs` - Module exports

### Built-in Profile YAMLs (9/9 complete)
All 9 profiles exist at `profiles/builtin/<name>/profile.yaml`:
- invoice, receipt, contract, scientific_paper, slide_deck
- form, bank_statement, legal_filing, book_chapter

### Profile READMEs (9/9 complete)
All 9 profiles have README.md at `profiles/builtin/<name>/README.md`

### Classifier Corpus (exists)
`tests/fixtures/classifier/` contains:
- contract, invoice, misc, scientific_paper directories
- MANIFEST.tsv
- README.md

### Fixtures Added (partial)
- invoice: 50 PDF fixtures ✓
- receipt: 2 PDF fixtures (needs 3 more)

## What Remains for `pdftract-1lp2` (Profile Authoring Epic)

### Missing Fixtures (per acceptance criteria: >= 5 per profile)
- bank_statement: 0/5 fixtures
- contract: 0/5 fixtures
- form: 0/5 fixtures
- receipt: 2/5 fixtures (needs 3 more)

### Missing Expected Output Files (0/9)
- `tests/fixtures/profiles/<name>/expected-output.json` does not exist for any profile
- These files contain the canonical `metadata.profile_fields` expected values for each fixture

### Missing Regression Tests (0/9)
- `tests/profiles/test_<name>.rs` does not exist for any profile
- Should run each fixture through `extract --profile <name>` and assert against expected-output.json

## Acceptance Criteria Status

For `pdftract-3a310` coordinator:

| Criterion | Status |
|-----------|--------|
| All Phase 7.10 child task beads closed | ❌ BLOCKED - `pdftract-1lp2` is open |
| Acrobat sample invoice classified > 0.8 confidence | ⚠️ NOT TESTED - needs classifier corpus run |
| Invoice field extraction >= 90% accuracy | ⚠️ NOT TESTED - needs expected-output.json + regression test |
| Custom profile with priority 100 overrides built-ins | ⚠️ NOT TESTED |
| Malformed regex profile rejected by validate | ⚠️ NOT TESTED |
| profile_fields.total: null when not found | ⚠️ NOT TESTED |
| Hot-reload picks up new YAML on next request | ⚠️ NOT TESTED |
| User profile shadowing shown in list | ⚠️ NOT TESTED |
| Built-in invoice profile >= 90% field accuracy | ⚠️ NOT TESTED |
| Field extraction adds < 5% to per-document time | ⚠️ NOT TESTED |
| 9 built-in profiles ship with >= 5 fixtures each | ❌ FAIL - bank_statement, contract, form have 0; receipt has 2 |
| Built-in profile YAML compiled via include_str! | ⚠️ NOT VERIFIED |

## Next Steps

To close `pdftract-3a310`, first close `pdftract-1lp2` (Profile Authoring epic):

1. Add missing fixtures (15 total: bank_statement 5, contract 5, form 5, receipt 3)
2. Generate expected-output.json for each profile's fixtures
3. Write regression tests at `tests/profiles/test_<name>.rs`
4. Run classifier corpus validation to verify >= 90% accuracy
5. Verify all acceptance criteria

## References

- Plan section: Phase 7.10 Document Profiles (lines 2890-3070)
- `pdftract-1lp2` (Profile Authoring epic) - must be closed first
- PROVENANCE.md at tests/fixtures/profiles/PROVENANCE.md (50KB, validates fixture sources)
