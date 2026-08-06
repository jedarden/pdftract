# Bead bf-3kzrqn: Add resolver context to Type3 rasterize function signature

## What was done

Verified that the document resolver context was already added to the Type3 rasterize function signature in bead bf-4zyfvd.

## Files examined

### crates/pdftract-core/src/font/type3_rasterizer.rs

- **DocumentContext struct** (lines 31-38): Defines the context structure with `source` field
- **rasterize_type3_glyph function signature** (lines 817-822): Already includes `doc_context: Option<&'a DocumentContext<'a>>` parameter
- The context is extracted on line 830: `let source = doc_context.and_then(|ctx| ctx.source);`

### crates/pdftract-core/src/font/resolver.rs

- **resolve_type3_level4 function** (line 624-746): Creates and passes the document context
  - Line 696: `let doc_ctx = Type3DocumentContext { source };`
  - Line 704: `rasterize_type3_glyph(font, &glyph_name, Some(&doc_ctx), Some(&callback))`
  - Line 707: Fallback with no context when unavailable

## Acceptance criteria verification

1. ✅ **Add resolver context parameter to type3_rasterize function signature**: Already present on line 820
2. ✅ **Pass context from caller through to the function**: Context passed on lines 704 and 707 in resolver.rs
3. ✅ **Function compiles successfully**: Verified with `cargo check --lib` - no errors
4. ✅ **Context is available for use in next step**: Context is accessible in function signature and can be used for char_proc_ref resolution

## Notes

The context parameter was added in bead bf-4zyfvd. This bead verified that the context is properly integrated and available for use in the next step of implementing char_proc_ref resolution for Type3 fonts.

The context is currently used to provide the `source` field to the `RasterizerContext` for form XObject resolution, and is ready to be used for resolving char_proc_ref in a future bead.
