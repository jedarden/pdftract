# Verification Note: bf-4zyfvd

## Task
Add document resolver context to Type3 rasterize function

## Changes Made

### 1. Added DocumentContext type to type3_rasterizer.rs
- Created `DocumentContext<'a>` struct with `source` field
- Added import for `PdfSource` trait

### 2. Updated rasterize_type3_glyph() signature
- Added `doc_context: Option<&'a DocumentContext<'a>>` parameter
- Added placeholder comment for future ObjRefPtr resolution
- Updated lifetime annotations to accommodate new parameter

### 3. Updated call site in resolver.rs
- Created `Type3DocumentContext` instance with `source` field
- Passed context to both conditional branches of `rasterize_type3_glyph()` call
- Added import for `DocumentContext` as `Type3DocumentContext`

### 4. Fixed test in resolver.rs
- Updated `test_resolve_type3_no_glyph` to pass required parameters to `resolve_type3()`

## Verification

### PASS Criteria
1. ✅ Added resolver context parameter to rasterize_type3_glyph() signature
2. ✅ Updated all call sites to pass the context
3. ✅ Compile succeeds with no errors (`cargo check --all-targets` passed)
4. ✅ Context is available inside the function for next step (TODO comment added)

### Files Modified
- `crates/pdftract-core/src/font/type3_rasterizer.rs`
- `crates/pdftract-core/src/font/resolver.rs`

### Next Steps
The document context is now available inside `rasterize_type3_glyph()` for use in resolving ObjRefPtr when implementing Type3 glyph content stream resolution. This enables the function to dereference object references during rasterization.
