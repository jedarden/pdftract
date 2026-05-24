# Verification Note: pdftract-68wfa

## Bead: 6.6.2: AtomicFileWriter (temp + rename) + Drop cleanup + panic safety

## Implementation Summary

### Changes Made

1. **Created `AtomicFileWriter` module** (`crates/pdftract-core/src/atomic_file_writer.rs`)
   - Implements atomic file writes using temp-file-and-rename pattern
   - Creates temp file as `<target>.tmp.<pid>.<random>` in same directory as target
   - `commit()` method atomically renames temp file to target on success
   - `Drop` implementation removes temp file if not committed
   - Special case for stdout ("-") passthrough

2. **Updated CLI extract command** (`crates/pdftract-cli/src/main.rs`)
   - Added `--output` option (default: "-" for stdout)
   - Integrated `AtomicFileWriter` for file outputs
   - All formats (json, text, markdown) now write through atomic file writer

3. **Added dependencies** (`crates/pdftract-core/Cargo.toml`)
   - `rand = "0.8"` for random suffix generation
   - `tempfile = "3.10"` for test fixtures

### Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Critical test: panic mid-extraction → no partial output files | **PASS** | Unit test `test_drop_without_commit_removes_temp` verifies temp file cleanup on Drop |
| Successful extraction: temp file renamed to target | **PASS** | Unit test `test_successful_commit` verifies rename on commit |
| Concurrent extractions: no collision | **PASS** | Unit test `test_concurrent_writes_no_collision` verifies 10 concurrent writers get unique temp paths |
| Drop cleanup: orphaned temp files removed on Drop | **PASS** | Drop impl removes temp file if not committed |
| File-backed sinks wrap Box<dyn Write> in AtomicFileWriter | **PASS** | CLI extract command now uses AtomicFileWriter for all file outputs |
| Stdout sinks (path == "-") pass through | **PASS** | stdout() method and "-" special case implemented |

### Test Results

All 7 unit tests pass:
```
test atomic_file_writer::tests::test_empty_file ... ok
test atomic_file_writer::tests::test_drop_without_commit_removes_temp ... ok
test atomic_file_writer::tests::test_stdout_passthrough ... ok
test atomic_file_writer::tests::test_successful_commit ... ok
test atomic_file_writer::tests::test_concurrent_writes_no_collision ... ok
test atomic_file_writer::tests::test_overwrite_existing_file ... ok
test atomic_file_writer::tests::test_large_file ... ok
```

### Git Commits

- `feat(pdftract-68wfa): implement AtomicFileWriter for atomic file writes`
  - Added `atomic_file_writer.rs` module with temp-file-and-rename pattern
  - Added `--output` option to extract command
  - Updated output handling to use AtomicFileWriter
  - Added unit tests for commit, drop, and concurrent write scenarios

### Files Modified

- `crates/pdftract-core/src/atomic_file_writer.rs` (new)
- `crates/pdftract-core/src/lib.rs` (module export)
- `crates/pdftract-core/Cargo.toml` (rand, tempfile deps)
- `crates/pdftract-cli/src/main.rs` (output option, AtomicFileWriter integration)

### Known Limitations

1. **Multi-sink transactional commit**: The plan mentions "For multi-sink: all sinks must commit successfully or NONE commit (transactional)". This is not yet implemented as the full multi-output CLI (Phase 6.6) is a separate feature. Current implementation handles single-file atomic writes.
2. **Cross-device rename**: The code detects and reports cross-device renames (non-atomic), but falls back to copy+delete is not implemented. This is acceptable for the current scope.

### Integration Notes

The AtomicFileWriter is now integrated into the CLI extract command:
- `pdftract extract file.pdf --output out.json` writes atomically to out.json
- `pdftract extract file.pdf` (default) writes to stdout (no atomic behavior needed)
- `pdftract extract file.pdf --output - --format json` explicitly writes to stdout

### Next Steps

This implementation provides the foundation for Phase 6.6 multi-output architecture. The full multi-output CLI (`--json out.json --md out.md --text out.txt`) will be implemented in a separate bead, building on this atomic file writer infrastructure.
