# Bead pdftract-22q8e: --highlight DIR annotated PDF writer

## Summary

Implemented the foundation for the `--highlight DIR` feature that writes annotated PDFs with /Highlight annotations for grep matches.

## What was implemented

### 1. Created `highlight.rs` module (crates/pdftract-cli/src/grep/highlight.rs)

- `group_matches_by_file_and_page()`: Groups match events by file and page for efficient batch writing
- `write_highlighted_pdfs()`: Main entry point that:
  - Groups matches by file
  - Generates output paths with collision handling (-1, -2 suffixes)
  - Calls per-file writer
- `write_single_highlighted_pdf()`: Placeholder that currently copies the file (full incremental update TODO)
- `create_highlight_annotation()`: Creates /Highlight annotation dict with:
  - /Type /Annot, /Subtype /Highlight
  - /Rect from match bbox
  - /QuadPoints [x0,y0, x1,y0, x1,y1, x0,y1] (BL, BR, TR, TL per PDF 1.7 spec)
  - /C [1.0, 1.0, 0.0] (yellow RGB)
  - /F 4 (print flag)
  - /CA 0.4 (opacity)
  - /T "pdftract grep" (author)
  - /Contents with match text

### 2. Module integration

- Added highlight module to `grep/mod.rs` with public exports
- Made progress module conditional on `grep` feature to fix compilation
- Fixed borrow issues in `worker.rs`

### 3. Tests

- `test_group_matches_by_file_and_page()`: Verifies correct grouping
- `test_group_matches_empty()`: Edge case handling
- `test_create_highlight_annotation()`: Verifies annotation structure

## Acceptance criteria status

### PASS
- Grouping logic correctly groups matches by file and page
- Annotation dictionary contains all required fields per PDF 1.7 spec 12.5.6.10
- /QuadPoints order follows spec (BL, BR, TR, TL)
- Output filename collision handling with -1/-2 suffixes
- Directory auto-creation via `create_dir_all` in `validate()`
- Module compiles without warnings

### WARN (known limitations)
- `write_single_highlighted_pdf()` currently does a simple file copy instead of incremental update
- No actual annotation objects are written to the PDF yet
- No xref table update
- Cannot verify annotation count or round-trip extraction yet

### FAIL (not yet implemented)
- /Highlight annotation count in output matches MatchEvent count (needs full incremental update)
- Original PDF byte-identical to input (needs verification)
- Incremental-update structure verified by xref-table inspection (needs implementation)
- Encrypted PDFs skipped with diagnostic (needs implementation)
- Output validity testing (Acrobat, Chrome, etc.)

## Technical notes

The full incremental update implementation requires:
1. Parse xref table to find max object number
2. Create annotation dict objects with proper object numbers
3. Update page /Annots arrays (may need to create new page objects if /Annots is indirect)
4. Write new objects at end of file
5. Write new xref table and trailer with `/Prev` pointing to old xref offset

This is a significant undertaking that requires careful handling of:
- Object number allocation
- Dictionary vs indirect object references
- Xref table format (traditional vs stream)
- Trailer dictionary preservation

## Next steps for full implementation

1. Implement incremental PDF update writer in `write_single_highlighted_pdf()`
2. Add encrypted PDF detection and skip with diagnostic
3. Add verification tests (annotation count, xref inspection, round-trip extraction)
4. Add headless Chrome screenshot test for visual verification

## Files modified

- `crates/pdftract-cli/src/grep/highlight.rs` (new)
- `crates/pdftract-cli/src/grep/mod.rs`
- `crates/pdftract-cli/src/grep/worker.rs`

## Test results

- Library compiles successfully: `cargo check --package pdftract-cli --lib` ✓
- No clippy warnings in grep module ✓
- Tests pass for grouping and annotation creation (note: full integration tests blocked by pre-existing compilation errors in other modules)
