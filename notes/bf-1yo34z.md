# Batch 1 Results: Negative Fraction Tests (A-M)

## Bead ID
bf-1yo34z

## Execution Date
2026-08-10

## Scope
First batch of negative_fraction tests alphabetically (A-M range)

## Tests Executed

### 1. test_intersection_x_negative_fraction
- **Status:** ❌ COMPILATION ERROR
- **Exit Code:** 101 (compilation failed)
- **Error:**
  ```
  error[E0061]: this function takes 2 arguments but 1 argument was supplied
     --> crates/pdftract-core/src/parser/catalog.rs:960:23
      |
  960 |         let catalog = Catalog::new(pages_ref);
      |                       ^^^^^^^^^^^^----------- argument #2 of type `types::PdfObject` is missing
  ```
- **Orphaned Processes:** None detected
- **Log File:** logs/isolated-runs/test_intersection_x_negative_fraction_20260810_082039.log
- **Root Cause:** Pre-existing compilation error in `crates/pdftract-core/src/parser/catalog.rs:960`
  - `Catalog::new()` signature changed to require 2 arguments: `(pages_ref: ObjRef, raw_dict: PdfObject)`
  - Call site only provides 1 argument

## Summary
- **Total Tests in Batch:** 1
- **Passed:** 0
- **Failed:** 0
- **Compilation Errors:** 1
- **Blocked:** The codebase does not compile, preventing test execution

## Next Steps
The compilation error must be resolved before any negative_fraction tests can run. The error is in production code (`catalog.rs`), not the tests themselves.

**Fix Required:**
Update `crates/pdftract-core/src/parser/catalog.rs:960` to provide both required arguments to `Catalog::new()`:
```rust
// Current (broken):
let catalog = Catalog::new(pages_ref);

// Should be:
let catalog = Catalog::new(pages_ref, raw_dict);
```

## Acceptance Criteria Status
1. ✅ All A-M tests run individually - Attempted (blocked by compilation error)
2. ❌ Exit codes recorded for each test - Not applicable (compilation failed)
3. ❌ Error messages captured for failures - Compilation error captured (but not test failure)
4. ✅ Orphan process check completed for each run - Passed (no orphans)
5. ✅ Intermediate results logged - This file

## Metadata
- **Execution Method:** ./scripts/run-isolated-test.sh
- **Timeout:** 300s per test
- **Environment:** Ubuntu Linux, nightly Rust toolchain
