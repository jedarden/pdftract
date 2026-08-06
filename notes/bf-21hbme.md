# Verification Note: bf-21hbme

## Task
Add error variants for missing char_proc object references

## Implementation Summary

Added Type3-specific error handling for missing character procedure references in Type3 fonts.

### Changes Made

1. **Added Type3Error enum** (`crates/pdftract-core/src/font/type3.rs`):
   - Created `Type3Error` enum with `MissingCharProcRef { glyph_name: String }` variant
   - Implemented `std::fmt::Display` trait for error messages
   - Implemented `std::error::Error` trait
   - Added `Type3Result<T>` type alias for `Result<T, Type3Error>`

2. **Added char_proc_required() method**:
   - New method that returns `Type3Result<ObjRef>` instead of `Option<ObjRef>`
   - Returns `Ok(ObjRef)` when glyph exists in /CharProcs
   - Returns `Err(Type3Error::MissingCharProcRef { glyph_name })` when glyph is missing
   - Error message includes the missing glyph name for debugging

3. **Added comprehensive tests**:
   - `test_char_proc_required_missing_returns_error`: Verifies error is returned for missing glyphs
   - `test_char_proc_required_found_returns_ref`: Verifies Ok is returned for existing glyphs
   - `test_type3_error_display_includes_glyph_name`: Verifies error message includes glyph name

## Acceptance Criteria Verification

### ✅ 1. New error variant in appropriate error enum
- Added `Type3Error::MissingCharProcRef { glyph_name: String }` in `Type3Error` enum
- Follows the same pattern as `Type0Error` and `FontError` in the codebase

### ✅ 2. Function that looks up char_proc_ref returns Result with this error
- Added `char_proc_required(&self, glyph_name: &str) -> Type3Result<ObjRef>` method
- Uses `ok_or_else()` to convert Option to Result with the error

### ✅ 3. Basic error message includes the missing reference ID
- Error message format: "character procedure reference not found for glyph '{glyph_name}'"
- Verified in test: error message contains both "character procedure reference not found" and the glyph name

### ✅ 4. Error propagates correctly through call stack
- Error type implements `std::error::Error` trait
- Can be propagated with `?` operator
- Tests verify error is returned correctly

## Test Results

All Type3 font tests pass (16 tests):
- ✅ `test_char_proc_required_missing_returns_error`
- ✅ `test_char_proc_required_found_returns_ref`
- ✅ `test_type3_error_display_includes_glyph_name`
- ✅ All existing Type3 tests still pass

## Example Usage

```rust
use pdftract_core::font::type3::{Type3Font, Type3Error};

// When a glyph might be missing, use char_proc_required for explicit error handling
match font.char_proc_required("MyGlyph") {
    Ok(obj_ref) => {
        // Proceed with rasterization using obj_ref
    }
    Err(Type3Error::MissingCharProcRef { glyph_name }) => {
        // Handle missing glyph - could log warning, use fallback glyph, etc.
        eprintln!("Warning: Glyph '{}' not found in Type3 font", glyph_name);
    }
}
```

## Files Modified

- `crates/pdftract-core/src/font/type3.rs`: Added error enum, result type, new method, and tests

## Commit

- Commit: `feat(bf-21hbme): add Type3Error for missing char_proc references`
