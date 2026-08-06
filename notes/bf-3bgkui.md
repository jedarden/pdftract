# Investigation: Execution Results Structure for Glyph Data

**Bead ID:** bf-3bgkui
**Date:** 2026-08-06
**Investigator:** Claude Code (GLM-4.7)

## Summary

This investigation examined the execution results structure to understand what glyph/path data is available for downstream processing. The investigation covered both internal data structures and external JSON schema outputs.

## 1. Internal Execution Result Structure

### ExecutionResult (content_stream.rs:781-792)

The primary execution result type returned by `execute_with_mode()`:

```rust
pub struct ExecutionResult {
    /// Glyphs extracted from the content stream.
    pub glyphs: Vec<Glyph>,
    /// Image XObjects encountered via Do operator (for Phase 4.4 figure detection).
    pub images: Vec<ImageXObject>,
    /// Diagnostics emitted during execution.
    pub diagnostics: Vec<Diagnostic>,
}
```

### Glyph Structure (glyph/mod.rs:70-99)

The core glyph structure is the primary unit of text extraction:

```rust
pub struct Glyph {
    /// Resolved Unicode codepoint (U+FFFD on failure, never panics).
    pub codepoint: char,
    /// Source of the Unicode mapping (ToUnicode, AGL, Fingerprint, ShapeMatch, Unknown).
    pub unicode_source: UnicodeSource,
    /// Confidence score [0.0, 1.0] derived from unicode_source.
    pub confidence: f32,
    /// Bounding box in PDF user space [x0, y0, x1, y1] (lower-left origin, y-axis UP).
    pub bbox: [f32; 4],
    /// Font name (shared via Arc across all glyphs of same font on the page).
    pub font_name: Arc<str>,
    /// Font size in points.
    pub font_size: f32,
    /// Text rendering mode (0-7 per PDF spec).
    pub rendering_mode: u8,
    /// Fill color (boxed to reduce Glyph struct size).
    pub fill_color: Box<Color>,
    /// Synthetic word boundary flag (true when TJ kerning injects space before this glyph).
    pub is_word_boundary: bool,
    /// Marked Content Identifier (MCID) from innermost BDC frame.
    pub mcid: Option<u32>,
    /// OCG hidden flag (true if glyph is within a default-OFF Optional Content Group).
    pub is_hidden: bool,
}
```

**Key fields for path/shape data:**
- `bbox: [f32; 4]` - **Primary path data field** containing bounding box coordinates
- `codepoint: char` - Unicode value for the glyph
- `font_name: Arc<str>` - Font identifier
- `font_size: f32` - Size for scaling calculations

### ImageXObject Structure (content_stream.rs:211-226)

```rust
pub struct ImageXObject {
    /// Bounding box in PDF user-space points [x0, y0, x1, y1].
    pub bbox: [f32; 4],
    /// The XObject reference.
    pub xobject_ref: ObjRef,
    /// The XObject name (for diagnostics).
    pub name: Arc<str>,
}
```

## 2. Coordinate System and Bbox Format

### Bounding Box Format

All bbox fields use the format: `[x0, y0, x1, y1]`
- **x0, y0**: Bottom-left corner
- **x1, y1**: Top-right corner
- **Coordinate system**: PDF user space (lower-left origin, y-axis UP)
- **Units**: Points (1/72 inch)

### Rotation Normalization

Per INV-30: Glyph bboxes are in PDF user space **AFTER /Rotate normalization**. The `normalize_glyph_bboxes_by_rotation()` function applies inverse rotation matrices to ensure downstream layout phases always operate in an un-rotated coordinate system.

### Bbox Computation

The `compute_device_bbox()` function (glyph/mod.rs:312-394) computes device-space bboxes by:

1. Getting glyph bbox in font units from font metrics
2. Scaling by `font_size/1000` to get text-space bbox
3. Applying `text_rise` (Ts) as y offset
4. Transforming all 4 corners by `text_matrix`
5. Transforming by CTM (Current Transformation Matrix)
6. Computing axis-aligned bbox from transformed corners

## 3. Color Data

### Color Enum (graphics_state.rs)

```rust
pub enum Color {
    /// DeviceGray: single component 0.0–1.0 (black to white)
    DeviceGray(f32),
    /// DeviceRGB: three components [R, G, B] each 0.0–1.0
    DeviceRGB([f32; 3]),
    /// DeviceCMYK: four components [C, M, Y, K] each 0.0–1.0
    DeviceCMYK([f32; 4]),
    /// Spot color: (colorant name, tint 0.0–1.0)
    Spot(Arc<str>, f32),
    /// Other color spaces (CalRGB, ICCBased, Pattern, etc.) — not serializable to CSS
    Other,
}
```

### Color Serialization

The `Glyph::fill_color_css()` method returns `Option<String>`:
- **Some(hex)**: For DeviceGray, DeviceRGB, DeviceCMYK (convertible to CSS hex)
- **None**: For Spot and Other color spaces (not serializable to CSS)

## 4. Unicode Source Tracking

### UnicodeSource Enum

The `unicode_source` field indicates how the Unicode mapping was derived:

- **ToUnicode**: High confidence (0.9-1.0) - Direct CMap lookup
- **Agl**: Medium confidence (0.5-0.9) - Adobe Glyph List fallback
- **Fingerprint**: Medium confidence (0.5-0.9) - Font fingerprinting
- **ShapeMatch**: Low-medium confidence (0.0-0.5) - Visual shape matching
- **Unknown**: Zero confidence (0.0) - Resolution failed, codepoint is U+FFFD
- **Ocr**: OCR-derived (confidence varies by OCR engine)

## 5. External JSON Schema (Output Format)

### SpanJson Schema (schema/mod.rs:63-136)

```rust
pub struct SpanJson {
    /// The extracted text content.
    pub text: String,
    /// Bounding box in PDF user-space points [x0, y0, x1, y1].
    pub bbox: [f64; 4],
    /// Font name or identifier.
    pub font: String,
    /// Font size in points.
    pub size: f64,
    /// Fill color as CSS hex string (e.g., "#000000").
    pub color: Option<String>,
    /// PDF Tr operator value (0-7).
    pub rendering_mode: Option<u8>,
    /// Confidence score (0.0 to 1.0).
    pub confidence: Option<f64>,
    /// Source: "vector", "ocr", "ocr-assisted", "ocr-fallback", "repaired".
    pub confidence_source: Option<String>,
    /// BCP-47 language tag if detected.
    pub lang: Option<String>,
    /// Style flags: "bold", "italic", "smallcaps", "subscript", "superscript".
    pub flags: Vec<String>,
    /// Column index (0-based) from Phase 4.3 column detection.
    pub column: Option<u32>,
    /// Optional cryptographic receipt.
    pub receipt: Option<Receipt>,
}
```

### BlockJson Schema (schema/mod.rs:174-200)

```rust
pub struct BlockJson {
    /// Block kind/type: "paragraph", "heading", "list", "table", "figure".
    pub kind: String,
    /// Concatenated text content of all spans.
    pub text: String,
    /// Bounding box in PDF user-space points [x0, y0, x1, y1].
    pub bbox: [f64; 4],
    /// Optional heading level (1-6) for heading blocks.
    pub level: Option<u8>,
    /// Optional table index for table blocks.
    pub table_index: Option<u32>,
    /// Indices of spans in this block.
    pub spans: Vec<usize>,
    /// Optional cryptographic receipt.
    pub receipt: Option<Receipt>,
}
```

## 6. Data Flow Summary

### Internal Processing Pipeline

1. **Phase 3** (content stream processing):
   - `execute_with_mode()` → `ExecutionResult`
   - Each glyph emitted via `emit_glyph()`
   - Bboxes computed via `compute_device_bbox()`

2. **Phase 4** (layout analysis):
   - Glyphs clustered into spans (`merge_glyphs_to_spans()`)
   - Spans grouped into blocks (`group_lines_into_blocks()`)
   - Column detection, reading order, table detection

3. **Output Generation**:
   - Internal `Glyph` → External `SpanJson`
   - Internal `Block` → External `BlockJson`
   - Serialization to JSON schema

### Accessibility Confirmed

**Acceptance Criteria Status:**

1. ✅ **Identified the execution results struct/type**
   - `ExecutionResult` (content_stream.rs:781-792)
   - Contains glyphs, images, diagnostics

2. ✅ **Located path/shape data fields within execution results**
   - Primary: `Glyph::bbox: [f32; 4]`
   - Secondary: `ImageXObject::bbox: [f32; 4]`
   - Both use `[x0, y0, x1, y1]` format (PDF user space, lower-left origin)

3. ✅ **Documented the data format (coordinates, commands, etc.)**
   - Bbox format: `[x0, y0, x1, y1]` (bottom-left to top-right)
   - Coordinate system: PDF user space (points, lower-left origin, y-axis UP)
   - Rotation normalization applied (INV-30)
   - Color encoding: DeviceGray/RGB/CMYK/Spot/Other (CSS hex when possible)

4. ✅ **Confirmed glyph bounds are accessible**
   - Bboxes computed via `compute_device_bbox()` (glyph/mod.rs:312-394)
   - Font metrics → text-space → device-space transformation
   - CTM and text_matrix transformations applied
   - Rotation normalization available via `normalize_glyph_bboxes_by_rotation()`

## 7. Key Files Referenced

- **Execution results**: `/home/coding/pdftract/crates/pdftract-core/src/content_stream.rs` (lines 781-792)
- **Glyph structure**: `/home/coding/pdftract/crates/pdftract-core/src/glyph/mod.rs` (lines 70-99)
- **Bbox computation**: `/home/coding/pdftract/crates/pdftract-core/src/glyph/mod.rs` (lines 312-394)
- **Color types**: `/home/coding/pdftract/crates/pdftract-core/src/graphics_state.rs`
- **JSON schema**: `/home/coding/pdftract/crates/pdftract-core/src/schema/mod.rs`
- **Extraction pipeline**: `/home/coding/pdftract/crates/pdftract-core/src/extract.rs`

## 8. Next Steps (for Parent Bead: bf-1a0two)

With the execution results structure now understood, the parent bead can proceed with implementing path/shape data extraction for vector header detection. The key insight is that **all glyph positional data is already available in the `Glyph::bbox` field** – no additional PDF parsing is required for basic position-based detection of headers over scanned content.

The glyph bbox data provides:
- Precise coordinates for vector text elements
- Font name and size for typography analysis
- Color information for visual distinction
- Confidence scores for reliability assessment

This data is sufficient to implement vector header detection logic that can distinguish between native PDF text (high confidence, precise bboxes) and scanned content (low confidence or OCR-assisted sources).
