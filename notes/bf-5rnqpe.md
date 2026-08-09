# Bead bf-5rnqpe: Verify cargo check passes for all targets

## Issue Found

**Compilation error:** `test_page_helper_error_handling.rs` failed to compile with error:
```
error[E0432]: unresolved import `pdftract_core::page_helper`
  --> crates/pdftract-core/tests/test_page_helper_error_handling.rs:17:20
   |
17 | use pdftract_core::page_helper::{self, PageError};
   |                    ^^^^^^^^^^^ could not find `page_helper` in `pdftract_core`
```

## Root Cause

The `page_helper.rs` module existed at `crates/pdftract-core/src/page_helper.rs` but was not exported in the library's public API (`lib.rs`).

## Fix Applied

Added `pub mod page_helper;` to `crates/pdftract-core/src/lib.rs` between `page_class` and `pages` modules to maintain alphabetical ordering.

```rust
pub mod page_class;
pub mod page_helper;  // <-- Added
pub mod pages;
```

## Verification

Ran `cargo check --all-targets` after the fix:
- **Status:** ✅ PASSED
- **Exit code:** 0 (success)
- **Compilation errors:** 0
- **Warnings:** 100+ (unused imports, dead code - acceptable for this bead per acceptance criteria)

## Acceptance Criteria Status

- ✅ `cargo check --all-targets` runs to completion
- ✅ No compilation errors (warnings are allowed for this bead)
- ✅ Issue documented in this verification note

## Files Modified

- `crates/pdftract-core/src/lib.rs` - Added `pub mod page_helper;` declaration

## Commit

Commit message: `fix(bf-5rnqpe): export page_helper module in lib.rs`
