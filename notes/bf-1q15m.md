# bf-1q15m: Validate grep-corpus exists before benchmark run

## Summary

Implemented comprehensive corpus validation for the `grep_1000` benchmark to verify the test data directory exists and contains valid PDF files before attempting to run the benchmark.

## Changes Made

### File: `crates/pdftract-cli/benches/grep_1000.rs`

1. **Added `get_corpus_files_dir()` function**: Returns the path to the `corpus/` subdirectory containing actual PDF files (previously code was checking wrong parent directory).

2. **Implemented `validate_corpus()` function**: Comprehensive validation that checks:
   - ✓ Corpus parent directory exists
   - ✓ `corpus/` subdirectory exists
   - ✓ Path is a directory (not a file)
   - ✓ Directory is readable
   - ✓ Contains at least one PDF file
   - ✓ All PDF files are readable and non-empty
   - ✓ Returns clear error messages for each failure case

3. **Updated `get_corpus_size()` and `count_corpus_files()`**: Changed to use `get_corpus_files_dir()` instead of `get_corpus_dir()` to count files in the correct subdirectory.

4. **Updated `run_benchmark()`**: Now calls `validate_corpus()` before proceeding with benchmark, preventing execution if corpus is missing or invalid.

5. **Updated `bench_grep_1000()` test**: Now uses validation and skips with clear error message if corpus is not available.

## Acceptance Criteria

- ✅ **tests/fixtures/grep-corpus/corpus/ directory exists**: Verified - contains 1000 PDF files
- ✅ **Directory contains at least one test file**: Verified - contains 1000 PDF files
- ✅ **All files are readable**: Verified - validation checks file metadata and readability
- ✅ **Validation returns success or clear error message**: Implemented with detailed error messages for each failure case
- ✅ **No benchmark run attempted if validation fails**: Implemented - `validate_corpus()` returns `Result` and `run_benchmark()` exits early on error

## Testing

Created and ran standalone test program (`test_corpus_validation.rs`) to verify validation logic:

**Success case** (corpus exists):
```
✓ Corpus parent directory exists
✓ Corpus subdirectory exists
✓ Corpus path is a directory
✓ Corpus directory is readable
✓ Corpus validation passed: 1000 PDF files
```

**Error case 1** (missing corpus directory):
```
✗ FAILED: Corpus validation failed
Error: Corpus directory not found: "/nonexistent/path"
```

**Error case 2** (missing corpus subdirectory):
```
✗ Corpus parent directory exists
✗ FAILED: Corpus validation failed
Error: Corpus files subdirectory not found: "/tmp/corpus"
```

## Compilation

- ✅ Benchmark compiles successfully: `cargo check --bench grep_1000` (no output = success)

## Impact

This change prevents the benchmark from running with invalid or missing test data, which would produce misleading results or waste CI resources. The validation provides clear error messages guiding users to regenerate the corpus if needed.

## Related Files

- `tests/fixtures/grep-corpus/corpus/` - Contains 1000 synthetic PDF files
- `tests/fixtures/grep-corpus/manifest.csv` - File metadata
- `tests/fixtures/grep-corpus/README.md` - Corpus documentation
