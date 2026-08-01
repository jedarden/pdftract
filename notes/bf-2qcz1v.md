# Bead bf-2qcz1v: Run truncated-flate test in isolation

## Task
Run the truncated-flate test standalone to verify the new STREAM_DECODE_ERROR assertion works.

## Execution

### Test Command
```bash
cargo test -p pdftract-core --test test_truncated_flate_recovery
```

### Test Results
```
running 9 tests
test test_truncated_flate_emits_diagnostics ... ok
test test_truncated_flate_emits_stream_decode_error ... FAILED
test test_truncated_flate_extraction_result_structure ... ok
test test_truncated_flate_extract_page_returns_result ... ok
test test_truncated_flate_fixture_exists ... ok
test test_truncated_flate_materialize_pages ... ok
test test_truncated_flate_opens_with_extractor ... ok
test test_truncated_flate_parses_as_pdf ... FAILED
test test_truncated_flate_partial_content_accessible ... FAILED

test result: FAILED. 6 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
```

## Analysis

### Key Failure: test_truncated_flate_emits_stream_decode_error

The critical test that validates the STREAM_DECODE_ERROR assertion **FAILED**:

**Expected behavior:**
- The test expects a `STREAM_DECODE_ERROR` diagnostic to be emitted when processing `truncated-flate.pdf`
- The fixture contains a truncated FlateDecode stream that should trigger the diagnostic

**Actual behavior:**
```
Testing STREAM_DECODE_ERROR emission for: /home/coding/pdftract/crates/pdftract-core/../../tests/fixtures/malformed/truncated-flate.pdf
✓ extract_pdf() succeeded
  Fingerprint: pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
  Page count: 0
  Total diagnostics: 0

thread 'test_truncated_flate_emits_stream_decode_error' (1372103) panicked at crates/pdftract-core/tests/test_truncated_flate_recovery.rs:389:5:
Expected STREAM_DECODE_ERROR diagnostic not found. Got 0 diagnostics: []
```

**Root cause:**
- `extract_pdf()` succeeded without errors
- **0 diagnostics were emitted** (expected at least 1 STREAM_DECODE_ERROR)
- The STREAM_DECODE_ERROR assertion is **NOT executing** or not being triggered

### Secondary Failures

The other two test failures are cascading consequences:
1. `test_truncated_flate_parses_as_pdf` - Expects at least one page, but got 0 pages
2. `test_truncated_flate_partial_content_accessible` - Also expects pages, but got 0 pages

These failures are downstream effects of the primary issue: the truncated stream is causing complete page loss rather than emitting diagnostics and attempting recovery.

## Acceptance Criteria Status

- ❌ **Test passes (exit code 0)**: NO - Test exited with code 101 (3 failed tests)
- ❌ **Assertion executes during test run**: NO - The STREAM_DECODE_ERROR diagnostic was not emitted
- ✅ **No test hangs or crashes**: YES - Test completed without hanging

## Conclusion

**The test revealed that the STREAM_DECODE_ERROR assertion is NOT working as intended.**

The fixture file (`truncated-flate.pdf`) contains a truncated FlateDecode stream, but the extraction:
1. Succeeded without returning an error
2. Did NOT emit any diagnostics (expected STREAM_DECODE_ERROR)
3. Returned 0 pages (suggesting total failure rather than graceful degradation)

This indicates the implementation from the previous bead (bf-2h1nt) may have issues:
- The assertion may not be properly placed in the stream decoder
- The error detection logic may not be triggering on truncated streams
- The diagnostic emission may be silently failing

## Recommendations

1. **Immediate**: Investigate why the FlateDecode decoder is not detecting the truncated stream
2. **Fix**: Ensure STREAM_DECODE_ERROR is emitted when flate2 returns an error
3. **Verify**: Re-run this test after fixes to confirm the diagnostic appears

## Test Environment
- Rust: latest stable
- Cargo test profile: unoptimized + debuginfo
- Test file: `crates/pdftract-core/tests/test_truncated_flate_recovery.rs`
- Fixture: `tests/fixtures/malformed/truncated-flate.pdf`

## Test Output Summary
- **Total tests**: 9
- **Passed**: 6
- **Failed**: 3
- **Critical failure**: `test_truncated_flate_emits_stream_decode_error` (the primary validation test)
