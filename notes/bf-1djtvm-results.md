# Comprehensive Test Results: Negative Fraction Tests

## Bead ID
bf-1djtvm

## Execution Date
2026-08-10

## Scope
Combined results from both test batches for negative_fraction tests. All 5 unique negative_fraction tests in the codebase were executed individually with isolated runs.

## Results Table

| Test Name | Exit Code | Status | Error Snippet | Orphan Status |
|-----------|-----------|---------|---------------|---------------|
| test_intersection_x_negative_fraction | 101 | ❌ COMPILATION ERROR | `error[E0061]: this function takes 2 arguments but 1 argument was supplied` at catalog.rs:960 | ✅ None |
| test_round_x_negative_fraction_rounds_down | 1 | ❌ COMPILATION ERROR | `error[E0061]: this function takes 2 arguments but 1 argument was supplied` at catalog.rs:960 | ✅ None |
| test_round_x_negative_fractions_round_down | 1 | ❌ COMPILATION ERROR | `error[E0061]: this function takes 2 arguments but 1 argument was supplied` at catalog.rs:960 | ✅ None |
| test_round_x_small_negative_fraction_rounds_down | 1 | ❌ COMPILATION ERROR | `error[E0061]: this function takes 2 arguments but 1 argument was supplied` at catalog.rs:960 | ✅ None |
| test_round_x_very_small_negative_fraction_rounds_down | 1 | ❌ COMPILATION ERROR | `error[E0061]: this function takes 2 arguments but 1 argument was supplied` at catalog.rs:960 | ✅ None |

## Summary Statistics

| Metric | Count |
|--------|-------|
| **Total Tests** | 5 |
| **Passed** | 0 |
| **Failed** | 0 |
| **Compilation Errors** | 5 (100%) |
| **Timeout** | 0 |
| **Orphaned Processes** | 0 |

## Observations and Patterns

### Critical Finding: Universal Compilation Error
All 5 negative_fraction tests are blocked by the **same pre-existing compilation error** in production code:
- **Location:** `crates/pdftract-core/src/parser/catalog.rs:960`
- **Error:** `Catalog::new()` requires 2 arguments `(pages_ref: ObjRef, raw_dict: PdfObject)` but only 1 argument is provided
- **Current Code:** `let catalog = Catalog::new(pages_ref);`
- **Required Fix:** `let catalog = Catalog::new(pages_ref, raw_dict);`

### Test Hygiene: Excellent Results
- ✅ **No orphaned processes** across all 5 isolated runs
- ✅ All tests executed with proper timeout guards (300s)
- ✅ Logs properly captured for each run in `logs/isolated-runs/`
- ✅ No test hangs or process leaks detected

### Pattern: Alphabetical Distribution
- **Batch 1 (A-M):** 1 test (`test_intersection_x_negative_fraction`)
- **Batch 2 (N-Z):** 4 tests (`test_round_x_*` variants)
- Total catalog of 5 unique negative_fraction tests

### Blocking Status
The compilation error prevents **any test execution** - these are not test failures, but a blocking infrastructure issue that must be resolved before:
1. Any negative_fraction tests can be validated
2. Any other tests that depend on `catalog.rs:960` can run
3. The codebase can be considered buildable

## Root Cause Analysis

### Technical Details
The `Catalog::new()` function signature was changed to require two parameters:
```rust
pub fn new(pages_ref: ObjRef, raw_dict: PdfObject) -> Self
```

However, a call site at `catalog.rs:960` was not updated:
```rust
let catalog = Catalog::new(pages_ref);  // Missing raw_dict argument
```

This is a **production code error**, not a test error. The tests are correct, but they cannot be executed because the codebase fails to compile.

## Next Steps

### Immediate Action Required
1. Fix the compilation error in `crates/pdftract-core/src/parser/catalog.rs:960`:
   ```rust
   let catalog = Catalog::new(pages_ref, raw_dict);
   ```

2. Verify the fix compiles successfully

3. Re-run all 5 negative_fraction tests to capture actual test results (PASS/FAIL rather than COMPILATION ERROR)

### Verification
After the compilation fix, re-execute:
```bash
./scripts/run-isolated-test.sh test_intersection_x_negative_fraction
./scripts/run-isolated-test.sh test_round_x_negative_fraction_rounds_down
./scripts/run-isolated-test.sh test_round_x_negative_fractions_round_down
./scripts/run-isolated-test.sh test_round_x_small_negative_fraction_rounds_down
./scripts/run-isolated-test.sh test_round_x_very_small_negative_fraction_rounds_down
```

## Related Beads
- **Parent:** bf-1gdxs9 (Negative Fraction Test Suite)
- **Batch 1:** bf-1yo34z (Tests A-M)
- **Batch 2:** bf-296336 (Tests N-Z)
- **Compilation Fix Needed:** This is a prerequisite for all dependent beads

## Metadata
- **Execution Method:** `./scripts/run-isolated-test.sh`
- **Timeout:** 300s per test
- **Environment:** Ubuntu Linux, nightly Rust toolchain
- **Log Directory:** `logs/isolated-runs/`
- **Total Execution Attempts:** 6 (5 unique tests × 1 attempt each, with 1 duplicate run)
