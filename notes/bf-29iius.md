# bf-29iius: Fix cache_cmd clear_cache() silently ignores file removal errors

## Summary

Fixed index inconsistency bug in `clear_cache()` and `purge_cache_older_than()` where file removal errors were silently ignored, leading to false reporting and index corruption.

## Changes Made

### 1. `clear_cache()` (lines 323-386)
- **Before:** Used `let _ = fs::remove_file()` and `let _ = fs::remove_dir()`, silently ignoring errors
- **After:**
  - Collects all removal errors with paths and error messages
  - Only increments `deleted` counter on successful removal
  - Returns error with detailed diagnostics if ANY file removal fails
  - **Does NOT reset index** (entry_count, hits, total_accesses) if removal failed
  - Displays up to 10 error paths with messages

### 2. `purge_cache_older_than()` (lines 390-478)
- **Before:** Same silent error handling pattern
- **After:**
  - Identical error collection and handling
  - Returns error if ANY removal fails
  - **Does NOT update index** if removal failed
  - Displays detailed error diagnostics

### 3. Test Coverage
Added `test_clear_cache_with_permission_failure()` (lines 689-740):
- Creates a test cache entry
- Simulates permission failure by making file read-only and directory non-writable (Unix)
- Verifies that:
  - `clear_cache()` returns `Err` when removal fails
  - Index is NOT reset (entry_count remains 1)
  - File still exists on disk
- On non-Unix systems, verifies successful cleanup

## Acceptance Criteria Verification

✅ **PASS:** fs::remove_file and fs::remove_dir errors are collected/counted
- Uses `match fs::remove_file()` with explicit `Ok(())` and `Err(e)` handling
- Maintains `failed` counter and `error_paths` Vec

✅ **PASS:** Functions return Err if ANY file removal fails
- Both functions check `if failed > 0` and return `bail!(...)` with detailed message

✅ **PASS:** Diagnostics emitted for each removal failure
- Up to 10 errors printed with path and error message
- "..." indicator if more than 10 errors

✅ **PASS:** Index NOT reset/set if any removal fails
- `clear_cache()`: Lines 375-386, index reset only happens after error check passes
- `purge_cache_older_than()`: Lines 468-478, index update only happens after error check passes

✅ **PASS:** Test simulates permission failure and verifies error handling
- Test creates entry, makes read-only, verifies failure, checks index unchanged

## Test Results

```
running 8 tests
test cache_cmd::tests::test_age_histogram_percentage ... ok
test cache_cmd::tests::test_age_histogram ... ok
test cache_cmd::tests::test_clear_cache_empty ... ok
test cache_cmd::tests::test_compute_stats_empty ... ok
test cache_cmd::tests::test_clear_cache_with_permission_failure ... ok
test cache_cmd::tests::test_count_entries ... ok
test cache_cmd::tests::test_compute_stats_with_entries ... ok
test cache_cmd::tests::test_count_entries_empty ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 346 filtered out
```

## Code Quality

- No breaking changes to existing API
- Error messages are informative and actionable
- Maintains backward compatibility for successful operations
- Uses existing error handling patterns (`anyhow::bail!`)
- Follows project conventions for Result<> handling

## Files Modified

- `crates/pdftract-cli/src/cache_cmd.rs`: ~110 lines changed (additions + modifications)
  - Lines 323-386: `clear_cache()` error handling
  - Lines 390-478: `purge_cache_older_than()` error handling  
  - Lines 689-740: New test `test_clear_cache_with_permission_failure()`
