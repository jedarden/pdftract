# Verification Note: pdftract-zl9y3

## Bead
**ID:** pdftract-zl9y3
**Title:** 7.5.1b: /AF associated files array walker (PDF 2.0 fallback to /EmbeddedFiles)

## Implementation Summary

### Files Created
- `crates/pdftract-core/src/attachment/mod.rs` - Attachment module root
- `crates/pdftract-core/src/attachment/associated_files.rs` - /AF array walker implementation (370 lines)

### Files Modified
- `crates/pdftract-core/src/lib.rs` - Added `pub mod attachment;` declaration

### Key Implementation Details

1. **`walk_af_array()` function**: Extracts `/AF` array from document catalog
   - Returns `Vec<AssociatedFileEntry>` with optional `/AFRelationship` and `filespec_ref`
   - Returns empty Vec for PDF 1.7 documents (no `/AF` key)
   - Emits `StructInvalidType` diagnostic if `/AF` is not an array
   - Skips non-Ref entries with diagnostic

2. **`AssociatedFileEntry` struct**: Represents a single /AF entry
   - `relationship: Option<String>` - /AFRelationship value (Source, Data, Alternative, Supplement, EncryptedPayload, Unspecified)
   - `filespec_ref: ObjRef` - Reference to the Filespec dictionary

3. **`extract_af_relationship()` helper**: Resolves Filespec and extracts `/AFRelationship`
   - Returns `Ok(Some(String))` if relationship present
   - Returns `Ok(None)` if absent (valid per spec)
   - Returns `Err` with diagnostics if resolution fails

### Acceptance Criteria Status

- [PASS] PDF 2.0 with /AF [filespec1, filespec2] → returns 2 entries (test: `test_walk_af_array_multiple_entries`)
- [PASS] PDF 1.7 with no /AF → empty Vec (test: `test_walk_af_array_empty`)
- [PASS] /AFRelationship preserved on output (test: `test_extract_af_relationship_present`, `test_walk_af_array_all_relationship_types`)
- [PASS] Non-array /AF → diagnostic emitted, returns Err (test: `test_walk_af_array_not_an_array`)
- [PASS] Non-Ref entry in /AF → diagnostic emitted, skips entry (test: `test_walk_af_array_non_ref_entry`)

### Test Results
All 12 unit tests pass:
- `test_associated_file_entry_new` - Entry construction
- `test_extract_af_relationship_present` - Relationship extraction
- `test_extract_af_relationship_absent` - No relationship (None)
- `test_extract_af_relationship_resolve_error` - Resolution failure
- `test_walk_af_array_empty` - PDF 1.7 (no /AF)
- `test_walk_af_array_single_entry` - Single entry with relationship
- `test_walk_af_array_multiple_entries` - Multiple entries
- `test_walk_af_array_no_relationship` - Entry without relationship
- `test_walk_af_array_not_an_array` - Invalid /AF type
- `test_walk_af_array_non_ref_entry` - Invalid entry type
- `test_walk_af_array_preserves_order` - Order preservation
- `test_walk_af_array_all_relationship_types` - All 6 PDF 2.0 relationship types

### Gates Passed
- [PASS] `cargo check --all-targets`
- [PASS] `cargo clippy -p pdftract-core --lib`
- [PASS] `cargo fmt -p pdftract-core --check`
- [PASS] `cargo test -p pdftract-core --lib attachment` (12/12 passed)

### Notes
- The `/EmbeddedFiles` name tree walker (sibling bead) is not yet implemented
- Merge with `/EmbeddedFiles` results will happen at the caller level when the sibling is complete
- All standard PDF 2.0 /AFRelationship values are supported: Source, Data, Alternative, Supplement, EncryptedPayload, Unspecified
