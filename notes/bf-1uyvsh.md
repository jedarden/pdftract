# bf-1uyvsh: Create test glyph infrastructure helper

## Summary

Created helper functions and minimal glyph data structures for testing Type3Font mock with rasterize_type3_glyph.

## Implementation

### Location
- `crates/pdftract-core/src/font/type3_rasterizer.rs` (test module)
- New submodule: `glyph_helpers`

### Helper Functions Created

1. **`create_char_procs(glyphs: &[(&str, u32)]) -> HashMap<Arc<str>, ObjRef>`**
   - Creates char_procs HashMap for Type3Font::mock()
   - Maps glyph names to object references

2. **`rectangle_glyph(x, y, width, height) -> Vec<u8>`**
   - Generates PDF content stream for filled rectangle
   - Uses "re" (rectangle) and "f" (fill) operators

3. **`line_glyph(x0, y0, x1, y1) -> Vec<u8>`**
   - Generates PDF content stream for stroked line
   - Uses "m" (move-to), "l" (line-to), and "s" (stroke) operators

4. **`triangle_glyph(x0, y0, x1, y1, x2, y2) -> Vec<u8>`**
   - Generates PDF content stream for filled triangle
   - Uses "m", "l", "h" (close-path), and "f" operators

5. **`create_simple_resolver(streams: &[(u32, Vec<u8>)]) -> Box<StreamResolverFn>`**
   - Creates stream resolver callback for rasterize_type3_glyph
   - Maps object references to content stream bytes

6. **`create_minimal_doc_context() -> DocumentContext<'static>`**
   - Returns minimal DocumentContext with None fields

7. **`create_test_setup(glyph_data: &[(&str, u32, Vec<u8>)]) -> (Type3Font, Box<StreamResolverFn>, DocumentContext<'static>)`**
   - Convenience function creating complete test setup
   - Returns font, resolver, and context ready for testing

### Tests Created

7 tests validating the helper functions:
- `test_glyph_helpers_create_char_procs` - Tests char_procs HashMap creation
- `test_glyph_helpers_rectangle_glyph` - Tests rectangle glyph generation
- `test_glyph_helpers_line_glyph` - Tests line glyph generation
- `test_glyph_helpers_triangle_glyph` - Tests triangle glyph generation
- `test_glyph_helpers_simple_resolver` - Tests resolver callback
- `test_glyph_helpers_complete_setup` - Tests complete test setup
- `test_glyph_helpers_integration_with_rasterize_type3_glyph` - Tests integration with rasterize_type3_glyph

## Usage Example

```rust
use pdftract_core::font::type3_rasterizer::tests::glyph_helpers::*;
use pdftract_core::font::type3::Type3Font;

// Create a test glyph setup
let (font, resolver, doc_context) = create_test_setup(&[
    ("rect1", 10, rectangle_glyph(0, 0, 100, 100)),
    ("line1", 11, line_glyph(10, 10, 50, 50)),
]);

// Rasterize a glyph
let bitmap = rasterize_type3_glyph(&font, "rect1", Some(&doc_context), Some(&resolver));
assert!(bitmap.is_some());
```

## Verification

### PASS Criteria Met
- ✅ Helper functions exist in test module
- ✅ Helper functions are documented with doc comments
- ✅ cargo test compiles with new helper functions
- ✅ All 7 new tests pass
- ✅ Helper is compatible with Type3Font::mock output structure
- ✅ Integration test with rasterize_type3_glyph passes

### Test Results
```
test font::type3_rasterizer::tests::test_glyph_helpers_create_char_procs ... ok
test font::type3_rasterizer::tests::test_glyph_helpers_complete_setup ... ok
test font::type3_rasterizer::tests::test_glyph_helpers_line_glyph ... ok
test font::type3_rasterizer::tests::test_glyph_helpers_rectangle_glyph ... ok
test font::type3_rasterizer::tests::test_glyph_helpers_simple_resolver ... ok
test font::type3_rasterizer::tests::test_glyph_helpers_triangle_glyph ... ok
test font::type3_rasterizer::tests::test_glyph_helpers_integration_with_rasterize_type3_glyph ... ok

test result: ok. 7 passed; 0 failed; 0 ignored
```

### Compilation
- Code compiles without errors
- Fixed compilation issue: `obj_ref.object` field name (was `obj_ref.obj`)

## Files Modified
- `crates/pdftract-core/src/font/type3_rasterizer.rs` - Added glyph_helpers submodule with 7 helper functions and 7 tests

## Commit
Commit: [COMMIT_HASH_PLACEHOLDER]
