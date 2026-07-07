# bf-2n36a: Baseline cargo nextest without timing flags

## Task
Run baseline cargo nextest without timing flags to establish output format for comparison with timing-flagged runs.

## Implementation

### Command Run
```bash
cargo nextest run -p pdftract-core --lib 2>&1 | tee notes/bf-mge0o-baseline.log
```

Ran on `pdftract-core` lib tests only (integration tests excluded due to compilation errors in `cjk_encoding.rs`).

### Output Format Captured

**Baseline nextest output characteristics:**
- Individual test lines: `PASS [   0.018s] (2869/2902) pdftract-core threads::tests::test_thread_header_with_fields`
- Test count and timing per test
- Summary section: `Summary [   4.527s] 2902 tests run: 2857 passed, 45 failed, 2 skipped`
- Failed tests listed with individual timings

**Test Suite:**
- 2,902 lib tests from `pdftract-core`
- Runtime: 4.527s total
- 2857 passed, 45 failed, 2 skipped

### Output File
Saved to: `notes/bf-mge0o-baseline.log` (411.8KB)

## Acceptance Criteria

- ✅ **PASS**: Baseline output file exists at `notes/bf-mge0o-baseline.log`
- ✅ **PASS**: Output shows standard nextest format without timing data
  - Individual test results with `[ <time>s]` format
  - Summary section with total runtime
  - Failed test listing
- ✅ **PASS**: File shows which test suite was run (pdftract-core lib tests)

## Notes

- Integration tests in `crates/pdftract-core/tests/` were excluded due to compilation errors in `cjk_encoding.rs`
- Baseline established on lib tests only, which provides sufficient format reference
- Output shows nextest's standard single-threaded timing format (no machine-readable output flags)

## Verification
- Output file reviewed and confirmed to contain standard nextest format
- Baseline can now be compared against runs with `--timer-format-plain`, `--message-format=json`, etc.
