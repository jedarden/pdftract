# Verification: Fix unused imports in outline.rs and pages.rs

## Task
Remove unused imports from `crates/pdftract-core/src/parser/outline.rs` and `crates/pdftract-core/src/parser/pages.rs`.

## Finding
**No unused imports found.** Both files are already clean.

## Verification Details

### outline.rs
All imports are actively used:
- `DiagCode`: 13 occurrences
- `Diagnostic`: 17 occurrences
- `ObjRef`: 80 occurrences
- `PdfObject`: 128 occurrences
- `PageDict`: 9 occurrences
- `XrefResolver`: 18 occurrences
- `HashSet`: 3 occurrences

### pages.rs
All imports are actively used:
- `DiagCode`: 19 occurrences
- `Diagnostic`: 30 occurrences
- `intern`: 91 occurrences
- `ObjRef`: 66 occurrences
- `PdfDict`: 33 occurrences
- `PdfObject`: 168 occurrences
- `merge_resources`: 3 occurrences
- `ResourceDict`: 5 occurrences
- `XrefResolver`: 22 occurrences
- `HashSet`: 7 occurrences
- `Arc`: 14 occurrences

### Build Status
- `cargo check --all-targets`: **PASS** (no warnings, no errors)
- No unused import warnings for outline.rs or pages.rs

## Root Cause Analysis
The categorization note (`notes/bf-4o94gz-categorization.md`) from 2026-08-09 reported 5 unused imports in outline.rs and 3 in pages.rs. However, these were likely cleaned up in earlier work (see bead bf-3easpf which documented "unused imports already fixed" for xref.rs and ocg.rs in the same batch).

## Outcome
**No file changes required.** Both files already have zero unused imports. The task is complete as-is.

## Test Results
- **PASS**: `cargo check --all-targets` returns clean (no warnings or errors)
- **PASS**: All imports in both files are actively used in the codebase

## Related Beads
- Parent bead: bf-5jt5tg (Fix unused imports in parser modules)
- Sibling bead: bf-3easpf (Fix unused imports in xref.rs and ocg.rs - already complete)
