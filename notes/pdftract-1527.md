# pdftract-1527: Shared conformance suite

## Summary

The shared SDK conformance suite at `tests/sdk-conformance/cases.json` was already created with 32 test cases covering all 9 contract methods. Fixed fixture paths to remove redundant "fixtures/" prefix.

## Work completed

### 1. Fixed fixture paths in cases.json

The fixture paths had an extra "fixtures/" prefix that caused validation to fail. Updated all paths to be relative to `tests/sdk-conformance/fixtures/`:

- `fixtures/misc/01.pdf` → `misc/01.pdf`
- `fixtures/encrypted/encrypted.pdf` → `encrypted/encrypted.pdf`
- `fixtures/scientific_paper/XX.pdf` → `scientific_paper/XX.pdf`
- etc.

### 2. Verified validation

All 32 test cases pass validation:
- extract: 8 cases (vector, scanned, encrypted, fillable-form, mixed, large, broken, remote)
- extract_text: 3 cases (unicode-heavy, vertical writing, math)
- extract_markdown: 3 cases (table-heavy, code-block, nested heading)
- extract_stream: 3 cases (page-at-a-time, cancellation, NDJSON format)
- search: 4 cases (literal, regex, case-insensitive, no-match)
- get_metadata: 3 cases (complete, minimal, XMP-only)
- hash: 2 cases (same file same hash, content stability)
- classify: 4 cases (academic, scientific, receipt, form)
- verify_receipt: 2 cases (valid, tampered)

## Acceptance criteria

| Criterion | Status | Notes |
|---|---|---|
| `tests/sdk-conformance/cases.json` exists with 30+ cases covering all 9 methods | PASS | 32 cases covering all methods |
| Each case has `id`, `fixture`, `method`, `options`, `expected`, `tolerances` fields | PASS | All required fields present |
| All fixtures referenced exist under `tests/sdk-conformance/fixtures/` | PASS | All fixtures found (symlinks + real files) |
| Cases tagged with optional `feature` and `min_schema_version` fields | PASS | All cases tagged appropriately |
| A schema-validation step validates the file on every commit | PASS | `validate_suite.py` validates JSON structure and fixtures |
| The Rust integration test suite consumes the same JSON file and passes 100% of cases | N/A | Implemented in sibling bead pdftract-1e5ud |
| Each SDK's conformance runner consumes this file and passes 100% before publishing | N/A | Implemented in sibling bead pdftract-5omc |

## Files changed

- `tests/sdk-conformance/cases.json` (fixed fixture paths)

## Retrospective

- **What worked:** The conformance suite was already well-structured with comprehensive coverage. The validation script made it easy to identify and fix the path issues.
- **What didn't:** N/A - straightforward path fix.
- **Surprise:** The fixture directory uses symlinks to share fixtures with the classifier tests, which is a good design choice to avoid duplication.
- **Reusable pattern:** When adding new fixtures, remember that paths in cases.json are relative to `tests/sdk-conformance/fixtures/`, not the workspace root.
