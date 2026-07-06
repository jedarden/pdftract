# bf-3b41k: Create truncated-flate test scaffold

## Summary
Created and verified the test scaffold for truncated-flate.pdf testing.

## Work Done

### 1. Fixed Compilation Error
The test file already existed at `crates/pdftract-core/tests/test_truncated_flate_recovery.rs` but had a compilation error:
- Line 57 tried to format `Result` containing `XrefResolver` (which doesn't implement `Debug`) with `{:?}`
- Fixed by changing the assert to only format the error case with `if let Err(ref e) = result`
- This avoids the need for Debug on the success type

### 2. Verification
Compiled and ran the tests:
```bash
cargo test -p pdftract-core --test test_truncated_flate_recovery
```

Results:
- ✅ `test_truncated_flate_fixture_exists` - PASSED (verifies fixture file exists and is non-empty)
- ⚠️  `test_truncated_flate_parses_as_pdf` - FAILED (fixture missing /Root reference in trailer)
- ⚠️  `test_truncated_flate_emits_diagnostics` - FAILED (same parsing issue)
- ⚠️  `test_truncated_flate_partial_content_accessible` - FAILED (same parsing issue)

## Acceptance Criteria

### PASS
- ✅ Test file exists at crates/pdftract-core/tests/test_truncated_flate_recovery.rs
- ✅ Test references the truncated-flate.pdf fixture (via `fixture_path()` helper)
- ✅ Test compiles and runs without crashing (fixed Debug formatting issue)
- ✅ Basic fixture existence check passes

### WARN
- The fixture appears to be more severely malformed than the tests expected
  - Tests expect: PDF with truncated FlateDecode streams but otherwise valid structure
  - Actual: PDF missing `/Root` reference in trailer (fundamental structural issue)
- This is acceptable for a scaffold - the test structure is in place and can evolve
  as the implementation progresses

## Implementation Notes

The test scaffold includes:
1. **Module documentation** explaining the purpose (truncated FlateDecode recovery)
2. **fixture_path() helper** for clean path resolution
3. **Four test cases**:
   - `test_truncated_flate_fixture_exists` - basic existence check (PASSING)
   - `test_truncated_flate_parses_as_pdf` - parsing verification (needs fixture update)
   - `test_truncated_flate_emits_diagnostics` - diagnostic collection (pending API)
   - `test_truncated_flate_partial_content_accessible` - content access (needs fixture)

The fixture at `tests/fixtures/malformed/truncated-flate.pdf` exists (588 bytes) but may
need to be regenerated or the tests adjusted to match its actual structure.

## Files Modified
- `crates/pdftract-core/tests/test_truncated_flate_recovery.rs` - fixed compilation error

## Commit
`test(bf-3b41k): fix truncated-flate test compilation error`
