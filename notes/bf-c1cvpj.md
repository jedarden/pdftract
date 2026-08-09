# Review of `rasterize_type3_glyph` Function Signature and Dependencies

## Function Location
`crates/pdftract-core/src/font/type3_rasterizer.rs:1714`

## Function Signature

```rust
pub fn rasterize_type3_glyph<'a, R>(
    font: &Type3Font,
    glyph_name: &str,
    doc_context: Option<&'a DocumentContext<'a>>,
    resolve_stream: Option<&R>,
) -> Option<Vec<u8>>
where
    R: Fn(ObjRef) -> Option<Vec<u8>> + ?Sized,
```

## Parameters Explained

### 1. `font: &Type3Font`
**Type:** Reference to Type3Font struct  
**Purpose:** The Type3 font containing the glyph to rasterize  
**Required Data:**
- `char_procs: HashMap<Arc<str>, ObjRef>` - Maps glyph names to stream object references
- `font_bbox: [f32; 4]` - Font bounding box in glyph space [llx lly urx ury]
- `font_matrix: Matrix3x3` - Transform from glyph space to text space
- `raster_cache: Arc<DashMap<Arc<str>, Vec<u8>>>` - Cache for rasterized glyphs

**Source:** `crates/pdftract-core/src/font/type3.rs:51`

### 2. `glyph_name: &str`
**Type:** String slice  
**Purpose:** The name of the glyph to rasterize (e.g., "A", "B", "zero")  
**Validation:** Must exist in the font's `/CharProcs` dictionary

### 3. `doc_context: Option<&'a DocumentContext<'a>>`
**Type:** Optional reference to DocumentContext  
**Purpose:** Document resolver context for potential future use (form XObject resolution)  
**Structure:**
```rust
pub struct DocumentContext<'a> {
    pub resolver: Option<&'a XrefResolver>,  // PDF document resolver
    pub source: Option<&'a dyn PdfSource>,   // PDF source for reading streams
}
```
**Current Usage:** Passed for future use but not actively used in current implementation  
**Source:** `crates/pdftract-core/src/font/type3_rasterizer.rs:414`

### 4. `resolve_stream: Option<&R>`
**Type:** Optional reference to callback function  
**Signature:** `Fn(ObjRef) -> Option<Vec<u8>> + ?Sized`  
**Purpose:** Callback to resolve ObjRef to stream bytes  
**Behavior:**
- Takes an `ObjRef` (indirect object reference like "10 0 R")
- Returns `Some(Vec<u8>)` with decoded content stream bytes on success
- Returns `None` if resolution fails
- If `None` is provided as the callback, the function returns `None`

**Source:** `crates/pdftract-core/src/parser/object/types.rs:46`

## Return Type

**Type:** `Option<Vec<u8>>`  
**Values:**
- `Some(Vec<u8>)` - Grayscale bitmap bytes (0-255 values, row-major order)
  - 0 = black ink
  - 255 = white paper
  - Values in between = anti-aliased edges
- `None` - Glyph not found or stream resolution failed

**Bitmap Dimensions:** Calculated from `font.font_bbox` using `calculate_bitmap_dimensions`

## Success Indicators

1. **Glyph exists:** `font.char_procs(glyph_name)` returns `Some(ObjRef)`
2. **Stream resolves:** `resolve_stream` callback returns `Some(bytes)`
3. **Content executes:** Content stream parses and executes without fatal errors
4. **Rasterization completes:** `ctx.bitmap.as_bytes()` returns valid bitmap data

## Constraints and Prerequisites

### Required Prerequisites
1. **Type3Font must be loaded:** Font dictionary must be parsed via `Type3Font::load()`
2. **Glyph must exist:** `glyph_name` must be present in `font.char_procs`
3. **Stream resolver required:** `resolve_stream` callback must be provided (otherwise returns None)

### Optional Components
1. **DocumentContext:** Currently unused but reserved for future form XObject resolution
2. **FontBBox:** If not specified, defaults to [0, 0, 0, 0]

### Error Handling
- **Missing glyph:** Returns `None` (graceful degradation)
- **Stream resolution failure:** Returns `None`
- **Invalid content stream:** May produce partial bitmap or fail silently
- **No resolver provided:** Returns `None`

## Dependencies Summary

### Direct Dependencies
1. **Type3Font** - Font structure with CharProcs mapping
2. **ObjRef** - PDF indirect object reference
3. **DocumentContext** - Document resolver context (optional)
4. **RasterizerContext** - Internal rasterization context
5. **Bitmap** - Dynamic-sized bitmap structure

### Helper Functions Used
1. `font.char_proc(glyph_name)` - Get ObjRef for glyph name
2. `resolve_stream(char_proc_ref)` - Resolve reference to bytes
3. `RasterizerContext::new(font)` - Create rasterization context
4. `ctx.execute_content_stream(&bytes)` - Parse and execute content stream
5. `calculate_bitmap_dimensions(&font.font_bbox, None)` - Size bitmap
6. `ctx.bitmap.as_bytes()` - Extract bitmap bytes

### Related Types
- **Matrix3x3** - 3x3 transformation matrix
- **PdfSource** - Trait for reading PDF data
- **XrefResolver** - PDF cross-reference resolver
- **GraphicsState** - PDF graphics state
- **CurrentPath** - Path construction state

## Integration Notes

### For Testing
To call this function in tests, you need:
1. A `Type3Font` instance (can create minimal font with just CharProcs)
2. A glyph name that exists in the font
3. A stream resolver callback that returns mock or real content stream bytes

### For Production Use
To call this function in production:
1. Load Type3Font from PDF font dictionary
2. Provide real PDF resolver and source via DocumentContext
3. Implement stream resolver that decodes PDF streams
4. Handle `None` return for missing glyphs gracefully

### Current Limitations
1. **Form XObjects not implemented:** DocumentContext is passed but Do operator doesn't use it (see line 1039-1040)
2. **No diagnostics returned:** Function returns `Option<Vec<u8>>`, not `Result`, so diagnostics are lost
3. **Silent failures:** Content stream errors don't propagate to caller

## Verification Criteria Met

✅ Function signature is documented with all parameters explained  
✅ List of dependencies needed to call the function is identified  
✅ Expected return type and success indicators are documented  
✅ Notes on any constraints or prerequisites are recorded
