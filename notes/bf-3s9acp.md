# bf-3s9acp: Remove underscore prefix from doc_context parameter

## Changes Made

### File: `crates/pdftract-core/src/font/type3_rasterizer.rs`

- **Line 1049-1051**: Removed `let _doc_context = doc_context;` suppression line
- Replaced with comment indicating doc_context is now active and available for use

### Verification

✅ **Compilation**: `cargo check` passes with no errors

✅ **Call sites verified** (`crates/pdftract-core/src/font/resolver.rs`):
- Line 704: `rasterize_type3_glyph(font, &glyph_name, Some(&doc_ctx), Some(&callback))` ✓
- Line 707: `rasterize_type3_glyph(font, &glyph_name, None::<&Type3DocumentContext>, None::<&StreamResolverFn>)` ✓

Both call sites correctly pass the doc_context parameter (either Some(&doc_ctx) or None).

✅ **No other call sites need updating**: All calls to `rasterize_type3_glyph` already pass the parameter correctly.

✅ **No regression in non-Type3 code paths**: The change only affects Type3 rasterizer internal handling.

## Acceptance Criteria Status

- ✅ doc_context parameter no longer has underscore prefix (removed suppression binding)
- ✅ Call site in resolver.rs passes doc_context value (already passing correctly)
- ✅ Code compiles without errors
- ✅ No regression in non-Type3 code paths (Type3-specific change only)

## Notes

The parameter name in the function signature was already `doc_context` (not `_doc_context`). The only change needed was removing the local suppression binding `let _doc_context = doc_context;` which was hiding the unused parameter warning. This signals that the parameter is now active and available for future use (e.g., form XObject resolution).
