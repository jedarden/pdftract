# Survey of Existing Type3 Glyph Test Patterns

## Task Context
Survey existing test glyph patterns in the Type3Font test module before creating new helpers.

**Bead ID:** bf-5opwcl  
**Parent:** bf-1uyvsh  
**Date:** 2026-08-09

---

## Key Data Structures

### Type3Font (crates/pdftract-core/src/font/type3.rs:51-94)

```rust
pub struct Type3Font {
    /// /CharProcs dictionary: glyph name -> stream object reference
    pub char_procs: HashMap<Arc<str>, ObjRef>,
    
    /// /FirstChar and /LastChar: character code range for /Widths
    pub first_char: u8,
    pub last_char: u8,
    
    /// /Widths array: advance widths in glyph space
    pub widths: Vec<f64>,
    
    /// /FontMatrix: 3x3 transform from glyph space to text space
    pub font_matrix: Matrix3x3,
    
    /// /Resources: resource dictionary for glyph content streams
    pub resources: Option<Arc<PdfDict>>,
    
    /// /Encoding: code -> glyph name mapping
    pub encoding: FontEncoding,
    
    /// /FontBBox: font bounding box in glyph space [llx lly urx ury]
    pub font_bbox: [f32; 4],
    
    /// Diagnostics emitted during loading
    pub diagnostics: Vec<Diagnostic>,
    
    /// Rasterized glyph cache: glyph name -> dynamic bitmap
    pub raster_cache: Arc<DashMap<Arc<str>, Vec<u8>>>,
}
```

### RasterizerContext (crates/pdftract-core/src/font/type3_rasterizer.rs)

```rust
pub struct RasterizerContext {
    /// 32x32 grayscale bitmap for shape recognition
    bitmap: Bitmap32x32,
    
    /// Current graphics state
    gs: GraphicsState,
    
    /// Graphics state stack for q/Q operators
    gs_stack: GraphicsStateStack,
    
    /// Current path being constructed
    current_path: CurrentPath,
    
    /// Font reference for FontMatrix application
    font: Type3Font,
}
```

### Bitmap32x32

```rust
pub struct Bitmap32x32 {
    /// Fixed 32x32 grayscale bitmap (u8 per pixel)
    pixels: [u8; 1024],  // 32 * 32
}
```

---

## Existing Helper Functions

### Type3Font Creation

**`Type3Font::mock(char_procs: Option<HashMap<Arc<str>, ObjRef>>) -> Self`**  
Location: `type3.rs:499-513`

Creates a minimal Type3Font for testing with:
- Identity FontMatrix ([1 0 0 1 0 0]) - predictable coordinates
- FontBBox [0, 0, 1000, 1000] - standard glyph space
- StandardEncoding
- Empty or provided CharProcs
- Zero widths and char range
- No resources, no diagnostics
- Empty raster cache

**Usage pattern from tests:**
```rust
let mut char_procs = HashMap::new();
char_procs.insert(Arc::from("A"), ObjRef::new(10, 0));
char_procs.insert(Arc::from("B"), ObjRef::new(11, 0));
let font = Type3Font::mock(Some(char_procs));
```

### Glyph Query Functions

**`font.has_glyph(glyph_name: &str) -> bool`**  
Returns true if glyph exists in CharProcs.

**`font.char_proc(glyph_name: &str) -> Option<ObjRef>`**  
Returns ObjRef for glyph, or None if not found.

**`font.char_proc_required(glyph_name: &str) -> Type3Result<ObjRef>`**  
Returns Ok(ObjRef) or Err(Type3Error::MissingCharProcRef { glyph_name }).

**`font.glyph_count() -> usize`**  
Returns count of CharProcs entries.

---

## Input Format Expected by `rasterize_type3_glyph`

**Function signature:**
```rust
pub fn rasterize_type3_glyph<'a, R>(
    font: &Type3Font,
    glyph_name: &str,
    doc_context: Option<&'a DocumentContext<'a>>,
    resolve_stream: Option<&R>,
) -> Option<Vec<u8>>
where
    R: Fn(ObjRef) -> Option<Vec<u8>> + ?Sized
```

### Required Inputs:

1. **`font: &Type3Font`** - Type3Font instance with:
   - `char_procs` HashMap containing the glyph name
   - `font_matrix` for coordinate transformation
   - `font_bbox` for glyph space bounds

2. **`glyph_name: &str`** - Name of glyph to rasterize (e.g., "A", "B", "test")

3. **`resolve_stream: Option<&R>`** - Callback function that:
   - Takes `ObjRef` (from CharProcs)
   - Returns `Option<Vec<u8>>` (content stream bytes)
   - Must return `Some(bytes)` for rasterization to proceed
   - Returns `None` if resolution fails → function returns `None`

4. **`doc_context: Option<&DocumentContext>`** - Document context for form XObject resolution (currently unused)

### Output:

- `Some(Vec<u8>)` - 1024 bytes (32×32 grayscale bitmap, row-major)
- `None` - Glyph not found or stream resolution failed

---

## Current Test Glyph Creation Patterns

### Pattern 1: Minimal Mock (empty CharProcs)
```rust
let font = Type3Font::mock(None);
assert_eq!(font.glyph_count(), 0);
```

### Pattern 2: Mock with Named Glyphs
```rust
let mut char_procs = HashMap::new();
char_procs.insert(Arc::from("A"), ObjRef::new(10, 0));
char_procs.insert(Arc::from("B"), ObjRef::new(11, 0));
let font = Type3Font::mock(Some(char_procs));

assert!(font.has_glyph("A"));
assert_eq!(font.char_proc("A"), Some(ObjRef::new(10, 0)));
```

### Pattern 3: Stream Resolution via Closure
```rust
let font = Type3Font::mock(Some(char_procs));

// Resolver that returns content stream bytes
let resolver = |obj_ref: ObjRef| -> Option<Vec<u8>> { 
    Some(vec![])  // Empty stream for testing
};

let result = rasterize_type3_glyph(
    &font,
    "test",
    None,
    Some(&resolver),
);
```

### Pattern 4: Arbitrary Glyph Names
```rust
let mut char_procs = HashMap::new();
char_procs.insert(Arc::from("CustomGlyph1"), ObjRef::new(10, 0));
char_procs.insert(Arc::from("MySpecialGlyph"), ObjRef::new(11, 0));
```

---

## Test Content Stream Patterns

### Empty Stream (Default Bitmap)
```rust
let resolver = |_: ObjRef| -> Option<Vec<u8>> { Some(vec![]) };
// Produces all-white 32×32 bitmap
```

### Stream with PDF Operators
Content streams are byte arrays containing PDF drawing operators:
- Path construction: `m` (move), `l` (line), `c` (curve), `re` (rect)
- Painting: `f` (fill), `S` (stroke)
- Graphics state: `q` (save), `Q` (restore), `cm` (transform)

Example (from integration tests):
```rust
b"100 700 m 200 700 l 200 800 l 100 800 l f"  // Draw filled rectangle
```

---

## Helper Functions Found

### Type3Font Methods
- `mock()` - Test factory
- `has_glyph()` - Glyph existence check
- `char_proc()` - Get ObjRef for glyph
- `char_proc_required()` - Get ObjRef or error
- `glyph_count()` - Count glyphs
- `advance_for(code: u8)` - Get advance width in text space

### Bitmap32x32 Methods
- `white()` - Create all-white bitmap
- `black()` - Create all-black bitmap
- `get(x, y)` - Get pixel value (0-255)
- `set(x, y, value)` - Set pixel value
- `fill_rect(x0, y0, x1, y1, value)` - Fill rectangle
- `as_bytes()` - Get raw byte array

### Detection/Validation Functions (type3_rasterizer.rs)
- `detect_char_proc_type()` - Classify PDF object type
- `detect_char_proc_type_with_context()` - Classify with dereferencing
- `validate_char_proc_structure()` - Check required keys

---

## Key Observations

1. **No content stream builders**: Tests create minimal fonts but rely on external content streams (from real PDFs or manually written byte arrays)

2. **Mock-based testing**: `Type3Font::mock()` is the primary test factory

3. **Resolver callback pattern**: `rasterize_type3_glyph()` uses a closure for stream resolution, enabling test control

4. **Identity FontMatrix in tests**: Tests use identity matrix (no scaling) for predictable coordinates

5. **ObjRef-based glyph addressing**: Glyphs are identified by arbitrary ObjRef values in tests (e.g., `ObjRef::new(10, 0)`)

6. **Bitmap as primary output**: Rasterization produces 32×32 grayscale bitmaps for shape recognition

---

## Next Steps for Helper Creation

Based on this survey, new helpers should:

1. **Build content streams**: Create valid PDF operator sequences for common shapes (rectangles, lines, curves)

2. **Package as test fixtures**: Provide ready-to-use `(font, glyph_name, resolver)` tuples

3. **Support common glyphs**: Pre-build streams for standard glyphs (A, B, 0, 1, etc.)

4. **Abstract away ObjRef management**: Helpers should handle ObjRef generation internally

5. **Integrate with mock()**: Work seamlessly with `Type3Font::mock()` pattern

---

## References

- crates/pdftract-core/src/font/type3.rs:515-968 (test module)
- crates/pdftract-core/src/font/type3.rs:464-513 (mock function)
- crates/pdftract-core/src/font/type3_rasterizer.rs:1714-1748 (rasterize_type3_glyph)
- crates/pdftract-core/src/font/type3_rasterizer.rs:1750-1800 (bitmap tests)
