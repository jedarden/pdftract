# Discovery Error Analysis for bf-53y4ih

## Date: 2026-08-10

## Summary

**NO DISCOVERY ERRORS FOUND**

The test output file `tests/discovery-verification.txt` contains compilation errors, not test discovery errors. The test suite failed to compile, so it never reached the test discovery phase.

## Detailed Findings

### Discovery-Related Issues
**NONE DETECTED** - The following discovery error categories were searched for but not found:
- ❌ Duplicate test name errors
- ❌ "test should be a function" warnings  
- ❌ Test discovery warnings
- ❌ Tests skipped during discovery

### Actual Errors (Compilation Failures)

All errors are **compilation errors** in `crates/pdftract-core/src/document.rs`:

#### E0560: Missing struct fields (Lines 4794-4802)
```rust
error[E0560]: struct `catalog::Catalog` has no field named `uri`
error[E0560]: struct `catalog::Catalog` has no field named `direction`
error[E0560]: struct `catalog::Catalog` has no field named `lang`
error[E0560]: struct `catalog::Catalog` has no field named `view_prefs`
error[E0560]: struct `catalog::Catalog` has no field named `perms`
error[E0560]: struct `catalog::Catalog` has no field named `legal`
error[E0560]: struct `catalog::Catalog` has no field named `requirements`
error[E0560]: struct `catalog::Catalog` has no field named `collection`
error[E0560]: struct `catalog::Catalog` has no field named `needs_rendering`
```
**Location:** `crates/pdftract-core/src/document.rs:4794-4802`

#### E0308: Type mismatches (Lines 4803, 4855, 4902)
```rust
error[E0308]: mismatched types
  --> crates/pdftract-core/src/document.rs:4803-4804
     |
4803 |               raw_dict: Some(crate::parser::object::PdfObject::Dict(
     |                              -------------------------------------- arguments incorrect
4804 |                   crate::parser::object::PdfDict::new(),
     |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `Box<...>`, found `IndexMap<...>`
```
**Locations:** `crates/pdftract-core/src/document.rs:4803-4804, 4854-4855, 4901-4902`

#### E0382: Partial move error (Line 4944)
```rust
error[E0382]: use of partially moved value: `result`
  --> crates/pdftract-core/src/document.rs:4944
     |
4939 |             Err(DocumentError::PageOutOfBounds { source, requested, available }) => {
     |                                                  ------ value partially moved here
4944 |                 let msg = format!("{}", result.unwrap_err());
     |                                         ^^^^^^ value used here after partial move
```
**Location:** `crates/pdftract-core/src/document.rs:4939-4944`

### Compiler Warnings
- 286 warnings (137 duplicates) - mostly unused imports and unused variables
- These are **not** discovery errors; they're standard compiler lint warnings

## Conclusion

**Test discovery never occurred.** The compilation failed with 14 errors before the test harness could:
1. Collect test functions
2. Check for duplicate test names
3. Validate test function signatures
4. Build the test execution plan

**No bead closure recommended** - The task was to analyze discovery errors, and none exist. The errors found are compilation issues that prevent discovery from running.

## Recommendation

The compilation errors in `document.rs` must be fixed first before test discovery can occur. The issues are:
1. Struct field mismatches (possibly due to API changes)
2. Type boxing mismatches (`Box<IndexMap>` vs `IndexMap` vs `Arc<IndexMap>`)
3. Partial move in error handling pattern matching
