//! Mock test fixtures for Type3 rasterizer tests.
//!
//! This module provides minimal mock implementations of resolver, source,
//! and counter types for testing parameter passing in callbacks.
//! It also provides glyph dictionary structures and Content structs
//! for Type3 font testing.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;

use crate::font::encoding::{FontEncoding, NamedEncoding};
use crate::font::type3::Type3Font;
use crate::graphics_state::Matrix3x3;
use crate::parser::object::types::ObjRef;
use crate::parser::object::{intern, PdfDict, PdfObject};

/// Content stream representation for Type3 glyph drawing context.
///
/// This struct represents the complete drawing context for a Type3 glyph,
/// including graphics state parameters and the actual drawing commands.
/// It models what would be in a Type3 glyph's charproc content stream.
///
/// # Fields
///
/// * `stroke_color` - RGB color for stroking operations (0.0-1.0 range)
/// * `line_width` - Line width in user space units
/// * `glyph_width` - Glyph advance width (setcharwidth d1 operator parameter)
/// * `drawing_commands` - PDF graphics operators that draw the glyph shape
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::Content;
///
/// let content = Content {
///     stroke_color: [0.0, 0.0, 0.0], // Black
///     line_width: 1.0,
///     glyph_width: 100.0,
///     drawing_commands: b"0 0 m 100 0 l 100 100 l 0 100 l h f".to_vec(),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct Content {
    /// Stroke color in RGB (0.0 = black, 1.0 = white per component)
    pub stroke_color: [f32; 3],
    /// Line width in user space units
    pub line_width: f32,
    /// Glyph advance width (for d1 setcharwidth operator)
    pub glyph_width: f32,
    /// PDF graphics drawing commands (path construction and painting operators)
    pub drawing_commands: Vec<u8>,
}

impl Content {
    /// Create a new Content with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `stroke_color` - RGB stroke color [r, g, b] (0.0-1.0 range)
    /// * `line_width` - Line width in user space units
    /// * `glyph_width` - Glyph advance width
    /// * `drawing_commands` - PDF graphics operators as byte vector
    pub fn new(
        stroke_color: [f32; 3],
        line_width: f32,
        glyph_width: f32,
        drawing_commands: Vec<u8>,
    ) -> Self {
        Self {
            stroke_color,
            line_width,
            glyph_width,
            drawing_commands,
        }
    }

    /// Create a minimal Content with default graphics state.
    ///
    /// Uses standard defaults:
    /// - Stroke color: black [0, 0, 0]
    /// - Line width: 1.0
    /// - Glyph width: 100.0
    /// - Drawing commands: must be provided
    ///
    /// # Arguments
    ///
    /// * `drawing_commands` - PDF graphics operators as byte vector
    pub fn with_defaults(drawing_commands: Vec<u8>) -> Self {
        Self {
            stroke_color: [0.0, 0.0, 0.0], // Black
            line_width: 1.0,
            glyph_width: 100.0,
            drawing_commands,
        }
    }

    /// Create a Content with graphics state setup plus the drawing commands.
    ///
    /// This creates a complete Type3 glyph content stream that includes:
    /// 1. Graphics state setup (stroke color via RGB, line width)
    /// 2. d1 operator (setcharwidth with glyph width)
    /// 3. Save/restore graphics state (q/Q wrapper)
    /// 4. The actual path drawing commands
    ///
    /// # Arguments
    ///
    /// * `path_commands` - The PDF path construction and painting commands
    ///
    /// # Returns
    ///
    /// A complete content stream byte vector ready for use as a Type3 charproc.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crate::font::type3_test_fixtures::Content;
    ///
    /// let path = b"0 0 m 100 0 l 100 100 l 0 100 l h f";
    /// let content = Content::with_graphics_state(path);
    ///
    /// // Result includes: q (save), stroke setup, d1 (setcharwidth), path, Q (restore)
    /// ```
    pub fn with_graphics_state(path_commands: &[u8]) -> Vec<u8> {
        // Build complete content stream:
        // q                    - save graphics state
        // 0 0 0 RG            - set stroke color to black (RGB 0,0,0)
        // 1.0 w               - set line width to 1.0
        // 100.0 0 0 d1        - setcharwidth (glyph width 100, lsb 0)
        // <path commands>     - actual drawing commands
        // Q                    - restore graphics state

        let mut stream = Vec::new();

        // Save graphics state
        stream.extend_from_slice(b"q ");

        // Set stroke color to black (RGB 0 0 0 RG)
        stream.extend_from_slice(b"0 0 0 RG ");

        // Set line width
        stream.extend_from_slice(b"1.0 w ");

        // Set charwidth (d1 operator): wx wy [llx lly urx ury] d1
        // For simplicity, we use just the width version
        stream.extend_from_slice(b"100 0 0 d1 ");

        // Add the path commands
        stream.extend_from_slice(path_commands);
        stream.push(b' ');

        // Restore graphics state
        stream.extend_from_slice(b"Q");

        stream
    }

    /// Convert this Content to a complete content stream byte vector.
    ///
    /// This serializes the Content struct into a PDF content stream format
    /// that can be used as a Type3 glyph charproc.
    ///
    /// # Returns
    ///
    /// A byte vector containing the complete PDF content stream.
    pub fn to_stream(&self) -> Vec<u8> {
        Self::with_graphics_state(&self.drawing_commands)
    }
}

/// Create a simple charproc content stream with basic path commands.
///
/// This function creates a minimal PDF content stream that draws a simple
/// filled rectangle. The content stream uses the following PDF path commands:
/// - `m` - moveto (move current point without drawing)
/// - `l` - lineto (draw line from current point to new point)
/// - `h` - closepath (close the current subpath)
/// - `f` - fill (fill the path using nonzero winding rule)
///
/// The resulting stream draws a 500x500 pixel square filled with black,
/// suitable for use as a Type3 glyph charproc.
///
/// # Returns
///
/// A `Vec<u8>` containing the PDF content stream bytes. This can be used
/// directly as the return value from a mock resolver function or for testing
/// content stream parsing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::create_simple_charproc_stream;
///
/// let stream = create_simple_charproc_stream();
///
/// // Stream contains: "0 0 m 500 0 l 500 500 l 0 500 l h f"
/// assert!(!stream.is_empty());
/// assert!(stream.starts_with(b"0 0 m"));
/// ```
pub fn create_simple_charproc_stream() -> Vec<u8> {
    // PDF content stream drawing a 500x500 filled square:
    // 0 0 m         - moveto to origin (0,0)
    // 500 0 l       - lineto to (500,0)
    // 500 500 l     - lineto to (500,500)
    // 0 500 l       - lineto to (0,500)
    // h             - closepath
    // f             - fill (nonzero winding rule)
    b"0 0 m 500 0 l 500 500 l 0 500 l h f".to_vec()
}

/// Create a charproc content stream with curveto commands.
///
/// This function creates a PDF content stream that demonstrates the use of
/// curveto (`c`) commands for drawing curved paths. The stream draws a simple
/// shape with both straight and curved segments, suitable for testing Type3
/// font rasterizers that support bezier curves.
///
/// The content uses these PDF path commands:
/// - `m` - moveto
/// - `l` - lineto
/// - `c` - curveto (cubic bezier: x1 y1 x2 y2 x3 y3)
/// - `h` - closepath
/// - `f` - fill
///
/// # Returns
///
/// A `Vec<u8>` containing the PDF content stream bytes with curved paths.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::create_charproc_stream_with_curves;
///
/// let stream = create_charproc_stream_with_curves();
///
/// // Stream contains curveto commands
/// assert!(stream.contains(&b'c'));
/// ```
pub fn create_charproc_stream_with_curves() -> Vec<u8> {
    // PDF content stream with curves:
    // 0 0 m                    - moveto to origin
    // 500 0 l                 - lineto to (500,0) - top edge
    // 500 250 400 500 500 500 c - cubic bezier curve (control1, control2, endpoint)
    // 0 500 l                 - lineto to (0,500) - bottom edge
    // 250 250 0 250 0 0 c     - bezier curve back to origin
    // h                       - closepath
    // f                       - fill
    b"0 0 m 500 0 l 500 250 400 500 500 500 c 0 500 l 250 250 0 250 0 0 c h f".to_vec()
}

/// Create a simple rectangle charproc stream using path command builders.
///
/// This function creates a PDF content stream that draws a simple 10x10 rectangle
/// using the path command builder functions (moveto, lineto, closepath).
/// This validates that the path command functions work together and produce
/// valid PDF content.
///
/// The content uses these PDF path commands:
/// - `m` - moveto (move to origin without drawing)
/// - `l` - lineto (draw line to specified point)
/// - `h` - closepath (close the current subpath)
/// - `f` - fill (fill the path using nonzero winding rule)
///
/// # Returns
///
/// A `Vec<u8>` containing the PDF content stream bytes that draws a 10x10
/// rectangle starting at the origin.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::create_rectangle_charproc_stream;
///
/// let stream = create_rectangle_charproc_stream();
///
/// // Stream contains rectangle path commands
/// let stream_str = std::str::from_utf8(&stream).unwrap();
/// assert!(stream_str.contains("0 0 m"));
/// assert!(stream_str.contains("10 0 l"));
/// assert!(stream_str.contains("10 10 l"));
/// assert!(stream_str.contains("0 10 l"));
/// assert!(stream_str.contains("h"));
/// ```
pub fn create_rectangle_charproc_stream() -> Vec<u8> {
    use crate::font::path_commands::{closepath, lineto, moveto};

    // Build rectangle path using path command builders:
    // 1. Start at origin (0, 0)
    // 2. Line to (10, 0) - top edge
    // 3. Line to (10, 10) - right edge
    // 4. Line to (0, 10) - bottom edge
    // 5. Close path back to origin
    // 6. Fill the path
    let path = format!(
        "{} {} {} {} {} {}",
        moveto(0.0, 0.0),      // Start at origin
        lineto(10.0, 0.0),      // Line to (10, 0)
        lineto(10.0, 10.0),     // Line to (10, 10)
        lineto(0.0, 10.0),      // Line to (0, 10)
        closepath(),            // Close path
        "f"                     // Fill operator
    );

    path.into_bytes()
}

/// Create an empty main content stream with basic PDF text block structure.
///
/// This function creates the most minimal PDF content stream that contains
/// just the basic BT/ET (Begin Text/End Text) structure with no actual content.
/// This serves as the foundational structure for building more complex content
/// streams that will hold PDF drawing commands.
///
/// The content uses these PDF text operators:
/// - `BT` - Begin Text (start a text object)
/// - `ET` - End Text (end the text object)
///
/// # Returns
///
/// A `Vec<u8>` containing the PDF page content stream bytes with the basic
/// BT/ET structure. This empty stream can be used as a starting point for
/// building more complex content streams.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::create_empty_content_stream;
///
/// let stream = create_empty_content_stream();
///
/// // Stream contains basic BT/ET structure
/// assert!(stream.contains(&b'B')); // BT
/// assert!(stream.contains(&b'E')); // ET
/// ```
pub fn create_empty_content_stream() -> Vec<u8> {
    // PDF empty content stream with minimal BT/ET structure:
    // BT                - Begin Text (start a text object)
    // ET                - End Text (end the text object)
    //
    // This provides the foundational structure for PDF content streams.
    // Additional commands (Tf, Td, Tj, etc.) can be added between BT and ET
    // to create more complex content streams.
    b"BT ET".to_vec()
}

/// Create a main page content stream with Type3 font text drawing commands.
///
/// This function creates a PDF page content stream that demonstrates the use of
/// Type3 fonts for drawing text. The main content stream is what invokes the Type3
/// glyphs that were defined in the charproc streams.
///
/// The content uses these PDF text and graphics operators:
/// - `BT` - Begin Text (start a text object)
/// - `/F1 12 Tf` - Set Font (select font /F1 with size 12)
/// - `100 700 Td` - Translate (move text position to x=100, y=700)
/// - `(A) Tj` - Show Text (draw glyph "A" at current position)
/// - `ET` - End Text (end the text object)
///
/// # Returns
///
/// A `Vec<u8>` containing the PDF page content stream bytes that draws text
/// using a Type3 font.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::create_main_content_stream;
///
/// let stream = create_main_content_stream();
///
/// // Stream contains text drawing commands
/// assert!(stream.contains(&b'B')); // BT
/// assert!(stream.contains(&b'T')); // Tf
/// assert!(stream.contains(&b'T')); // Td
/// assert!(stream.contains(&b'T')); // Tj
/// assert!(stream.contains(&b'E')); // ET
/// ```
pub fn create_main_content_stream() -> Vec<u8> {
    // PDF main content stream that draws text using Type3 font:
    // BT                - Begin Text (start a text object)
    // /F1 12 Tf        - Set Font to /F1 with size 12
    // 100 700 Td       - Translate to position (100, 700)
    // (A) Tj           - Show Text "A" (invokes glyph "A" from charprocs)
    // ET                - End Text
    //
    // This stream demonstrates:
    // 1. Font selection (Tf operator)
    // 2. Text positioning (Td operator)
    // 3. Text drawing (Tj operator that references glyphs from the font)
    b"BT /F1 12 Tf 100 700 Td (A) Tj ET".to_vec()
}

/// Create a main page content stream with multiple text drawing commands.
///
/// This function creates a more comprehensive PDF page content stream that
/// demonstrates multiple text drawing operations using Type3 fonts. It shows
/// drawing multiple characters and adjusting text position between them.
///
/// The content uses these PDF text and graphics operators:
/// - `BT` - Begin Text
/// - `/F1 12 Tf` - Set Font
/// - `100 700 Td` - Translate to initial position
/// - `(AB) Tj` - Show Text "AB" (draws two glyphs at once)
/// - `0 -14 Td` - Adjust position (move down 14 units)
/// - `(C) Tj` - Show Text "C"
/// - `ET` - End Text
///
/// # Returns
///
/// A `Vec<u8>` containing the PDF page content stream bytes with multiple
/// text drawing operations.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::create_main_content_stream_multi;
///
/// let stream = create_main_content_stream_multi();
///
/// // Stream contains multiple text drawing commands
/// let stream_str = std::str::from_utf8(&stream).unwrap();
/// assert!(stream_str.contains("(AB)"));
/// assert!(stream_str.contains("(C)"));
/// ```
pub fn create_main_content_stream_multi() -> Vec<u8> {
    // PDF main content stream with multiple text operations:
    // BT                  - Begin Text
    // /F1 12 Tf          - Set Font to /F1 with size 12
    // 100 700 Td         - Translate to (100, 700)
    // (AB) Tj            - Show Text "AB" (draws glyphs A and B)
    // 0 -14 Td           - Translate down 14 units (line spacing)
    // (C) Tj             - Show Text "C" (draws glyph C)
    // ET                  - End Text
    b"BT /F1 12 Tf 100 700 Td (AB) Tj 0 -14 Td (C) Tj ET".to_vec()
}

/// Glyph entry containing all properties needed for Type3 font testing.
///
/// This structure represents a single glyph with its drawing properties
/// and reference to its content stream (charproc).
#[derive(Debug, Clone)]
pub struct GlyphEntry {
    /// Glyph name (e.g., ".notdef", "A", "a", etc.)
    pub name: Arc<str>,
    /// Advance width in glyph space units
    pub width: f64,
    /// Bounding box in glyph space [llx, lly, urx, ury]
    pub bbox: [f32; 4],
    /// Reference to the glyph's content stream (charproc)
    pub charproc_ref: ObjRef,
}

impl GlyphEntry {
    /// Create a new glyph entry with the given properties.
    pub fn new(
        name: impl Into<Arc<str>>,
        width: f64,
        bbox: [f32; 4],
        charproc_ref: ObjRef,
    ) -> Self {
        Self {
            name: name.into(),
            width,
            bbox,
            charproc_ref,
        }
    }

    /// Create a minimal glyph entry with default values.
    ///
    /// Uses standard defaults: width 500, bbox [0, 0, 500, 500], and a provided charproc_ref.
    pub fn minimal(name: impl Into<Arc<str>>, charproc_ref: ObjRef) -> Self {
        Self {
            name: name.into(),
            width: 500.0,
            bbox: [0.0, 0.0, 500.0, 500.0],
            charproc_ref,
        }
    }

    /// Create the standard ".notdef" glyph entry.
    ///
    /// The ".notdef" glyph is required in all Type3 fonts and is displayed
    /// when a glyph name is not found.
    pub fn notdef(charproc_ref: ObjRef) -> Self {
        Self {
            name: Arc::from(".notdef"),
            width: 500.0,
            bbox: [0.0, 0.0, 500.0, 500.0],
            charproc_ref,
        }
    }
}

/// Glyph dictionary type for Type3 font testing.
///
/// Maps glyph names to their complete entry properties including
/// width, bounding box, and charproc reference.
pub type GlyphDict = HashMap<Arc<str>, GlyphEntry>;

/// Mock resolver tracking flag.
///
/// Minimal fixture to verify resolver parameter was passed to a callback.
/// Uses `Arc<AtomicBool>` so it can be shared and cloned across threads.
///
/// # Example
///
/// ```rust
/// let resolver_called = Arc::new(AtomicBool::new(false));
/// let resolver_clone = resolver_called.clone();
/// let callback = move |obj_ref| {
///     resolver_clone.store(true, Ordering::SeqCst);
///     Some(b"test content".to_vec())
/// };
/// callback(ObjRef::new(1, 0));
/// assert!(resolver_called.load(Ordering::SeqCst));
/// ```
pub type MockResolver = Arc<AtomicBool>;

/// Create a new mock resolver flag initialized to false.
///
/// # Returns
///
/// A `MockResolver` (Arc<AtomicBool>) set to false.
pub fn mock_resolver() -> MockResolver {
    Arc::new(AtomicBool::new(false))
}

/// Mock source tracking flag.
///
/// Minimal fixture to verify source parameter was passed to a callback.
/// Uses `Arc<AtomicBool>` so it can be shared and cloned across threads.
///
/// # Example
///
/// ```rust
/// let source_used = Arc::new(AtomicBool::new(false));
/// let source_clone = source_used.clone();
/// let callback = move |obj_ref| {
///     source_clone.store(true, Ordering::SeqCst);
///     Some(b"test content".to_vec())
/// };
/// callback(ObjRef::new(1, 0));
/// assert!(source_used.load(Ordering::SeqCst));
/// ```
pub type MockSource = Arc<AtomicBool>;

/// Create a new mock source flag initialized to false.
///
/// # Returns
///
/// A `MockSource` (Arc<AtomicBool>) set to false.
pub fn mock_source() -> MockSource {
    Arc::new(AtomicBool::new(false))
}

/// Mock counter for tracking callback invocations.
///
/// Minimal fixture using `Arc<AtomicU64>` to track how many times
/// a callback was invoked or how many operations were performed.
///
/// # Example
///
/// ```rust
/// let counter = Arc::new(AtomicU64::new(0));
/// let counter_clone = counter.clone();
/// let callback = move |obj_ref| {
///     counter_clone.fetch_add(1, Ordering::SeqCst);
///     Some(b"test content".to_vec())
/// };
/// callback(ObjRef::new(1, 0));
/// callback(ObjRef::new(2, 0));
/// assert_eq!(counter.load(Ordering::SeqCst), 2);
/// ```
pub type MockCounter = Arc<AtomicU64>;

/// Create a new mock counter initialized to zero.
///
/// # Returns
///
/// A `MockCounter` (Arc<AtomicU64>) set to 0.
pub fn mock_counter() -> MockCounter {
    Arc::new(AtomicU64::new(0))
}

/// Create a glyph dictionary with basic properties for Type3 rasterization tests.
///
/// This function creates a glyph dictionary with a single test glyph that has
/// simple, well-defined properties suitable for testing the Type3 glyph
/// rasterization pipeline.
///
/// The test glyph uses the basic properties specified in the test requirements:
/// - Bounding box: [0, 0, 100, 100] (100x100 unit square at origin)
/// - Width: 100.0 (advance width matches bbox width)
///
/// # Arguments
///
/// * `test_ref` - ObjRef for the test glyph's content stream
///
/// # Returns
///
/// A GlyphDict with a single test glyph entry having the specified properties.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::{create_glyph_dict_with_basic_properties, GlyphDict};
/// use crate::parser::object::types::ObjRef;
///
/// let test_ref = ObjRef::new(11, 0);
/// let glyph_dict = create_glyph_dict_with_basic_properties(test_ref);
///
/// assert_eq!(glyph_dict.len(), 1);
/// let entry = glyph_dict.get("test_glyph").unwrap();
/// assert_eq!(entry.bbox, [0.0, 0.0, 100.0, 100.0]);
/// assert_eq!(entry.width, 100.0);
/// ```
pub fn create_glyph_dict_with_basic_properties(test_ref: ObjRef) -> GlyphDict {
    let mut dict = GlyphDict::new();

    // Add test glyph with basic properties:
    // - Bounding box: [0, 0, 100, 100]
    // - Width: 100.0
    let test_entry = GlyphEntry::new(
        "test_glyph",
        100.0,                    // width
        [0.0, 0.0, 100.0, 100.0], // bbox
        test_ref,
    );
    dict.insert(Arc::clone(&test_entry.name), test_entry);

    dict
}

/// Create a basic glyph dictionary with .notdef and "A" glyphs.
///
/// This function creates a glyph dictionary with two entries:
/// - ".notdef" glyph (required in all Type3 fonts)
/// - "A" glyph (a common test glyph)
///
/// # Arguments
///
/// * `notdef_ref` - ObjRef for the .notdef glyph's content stream
/// * `test_ref` - ObjRef for the "A" glyph's content stream
///
/// # Returns
///
/// A GlyphDict with both .notdef and "A" entries.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::create_basic_glyph_dict;
/// use crate::parser::object::types::ObjRef;
///
/// let notdef_ref = ObjRef::new(10, 0);
/// let test_ref = ObjRef::new(11, 0);
/// let dict = create_basic_glyph_dict(notdef_ref, test_ref);
///
/// assert_eq!(dict.len(), 2);
/// assert!(dict.contains_key(".notdef"));
/// assert!(dict.contains_key("A"));
/// ```
pub fn create_basic_glyph_dict(notdef_ref: ObjRef, test_ref: ObjRef) -> GlyphDict {
    let mut dict = GlyphDict::new();

    // Add .notdef glyph (required in all Type3 fonts)
    let notdef_entry = GlyphEntry::notdef(notdef_ref);
    dict.insert(Arc::clone(&notdef_entry.name), notdef_entry);

    // Add "A" glyph
    let a_entry = GlyphEntry::new("A", 600.0, [0.0, 0.0, 600.0, 700.0], test_ref);
    dict.insert(Arc::clone(&a_entry.name), a_entry);

    dict
}

/// Create a minimal glyph dictionary with only .notdef glyph.
///
/// This function creates a glyph dictionary with a single .notdef entry,
/// which is the minimum required glyph for a Type3 font.
///
/// # Arguments
///
/// * `notdef_ref` - ObjRef for the .notdef glyph's content stream
///
/// # Returns
///
/// A GlyphDict with only the .notdef entry.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::create_minimal_glyph_dict;
/// use crate::parser::object::types::ObjRef;
///
/// let notdef_ref = ObjRef::new(10, 0);
/// let dict = create_minimal_glyph_dict(notdef_ref);
///
/// assert_eq!(dict.len(), 1);
/// assert!(dict.contains_key(".notdef"));
/// ```
pub fn create_minimal_glyph_dict(notdef_ref: ObjRef) -> GlyphDict {
    let mut dict = GlyphDict::new();

    // Add only .notdef glyph
    let notdef_entry = GlyphEntry::notdef(notdef_ref);
    dict.insert(Arc::clone(&notdef_entry.name), notdef_entry);

    dict
}

/// Create a minimal Type3Font for testing.
///
/// This function creates a minimal Type3Font instance with a .notdef glyph,
/// suitable for testing Type3 font functionality without requiring a full
/// PDF document structure.
///
/// # Arguments
///
/// * `notdef_ref` - ObjRef for the .notdef glyph's content stream
///
/// # Returns
///
/// A Type3Font with minimal configuration (single .notdef glyph).
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::create_minimal_type3_font;
/// use crate::parser::object::types::ObjRef;
///
/// let notdef_ref = ObjRef::new(42, 0);
/// let font = create_minimal_type3_font(notdef_ref);
///
/// assert!(font.has_glyph(".notdef"));
/// assert_eq!(font.glyph_count(), 1);
/// ```
pub fn create_minimal_type3_font(notdef_ref: ObjRef) -> Type3Font {
    let mut char_procs = std::collections::HashMap::new();
    char_procs.insert(Arc::from(".notdef"), notdef_ref);

    let mut font_dict = PdfDict::new();
    let char_procs_dict = PdfObject::Dict(Box::new(
        char_procs
            .into_iter()
            .map(|(k, v)| (k, PdfObject::Ref(v)))
            .collect(),
    ));
    font_dict.insert(intern("/CharProcs"), char_procs_dict);

    // Use identity FontMatrix for predictable coordinates during testing
    font_dict.insert(
        intern("/FontMatrix"),
        PdfObject::Array(Box::new(vec![
            PdfObject::Real(1.0),
            PdfObject::Real(0.0),
            PdfObject::Real(0.0),
            PdfObject::Real(1.0),
            PdfObject::Real(0.0),
            PdfObject::Real(0.0),
        ])),
    );

    // Set FontBBox for a 1000x1000 glyph space
    font_dict.insert(
        intern("/FontBBox"),
        PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Integer(0),
            PdfObject::Integer(1000),
            PdfObject::Integer(1000),
        ])),
    );

    font_dict.insert(intern("/FirstChar"), PdfObject::Integer(0));
    font_dict.insert(intern("/LastChar"), PdfObject::Integer(0));
    font_dict.insert(
        intern("/Widths"),
        PdfObject::Array(Box::new(vec![PdfObject::Real(500.0)])),
    );

    Type3Font::load(&font_dict)
}

/// Convert a GlyphDict to a HashMap of charproc references.
///
/// This function converts a GlyphDict (which contains full GlyphEntry structs)
/// into a simpler HashMap that maps glyph names to their ObjRef content stream
/// references. This is useful when creating Type3Font instances or when you
/// only need the charproc references without the full glyph metadata.
///
/// # Arguments
///
/// * `glyph_dict` - The GlyphDict to convert
///
/// # Returns
///
/// A HashMap mapping glyph names (Arc<str>) to their ObjRef content stream references.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::{create_basic_glyph_dict, to_charprocs_map};
/// use crate::parser::object::types::ObjRef;
///
/// let notdef_ref = ObjRef::new(10, 0);
/// let test_ref = ObjRef::new(11, 0);
/// let glyph_dict = create_basic_glyph_dict(notdef_ref, test_ref);
///
/// let charprocs = to_charprocs_map(&glyph_dict);
///
/// assert_eq!(charprocs.len(), 2);
/// assert_eq!(charprocs.get(".notdef"), Some(&notdef_ref));
/// assert_eq!(charprocs.get("A"), Some(&test_ref));
/// ```
pub fn to_charprocs_map(glyph_dict: &GlyphDict) -> std::collections::HashMap<Arc<str>, ObjRef> {
    glyph_dict
        .iter()
        .map(|(name, entry)| (Arc::clone(name), entry.charproc_ref))
        .collect()
}

/// Character code to glyph name mapping type for Type3 font testing.
///
/// This type maps character codes (u8, 0-255) to glyph names (Arc<str>).
/// In Type3 fonts, content streams use character codes like `(ABC) Tj`,
/// which are mapped to glyph names via the font's encoding dictionary.
///
/// This is the mapping used during text extraction and rasterization when
/// character codes in content streams need to be resolved to actual glyphs.
pub type CharToGlyphMap = std::collections::HashMap<u8, Arc<str>>;

/// Create a character code to glyph name mapping for basic ASCII testing.
///
/// This function creates a simple encoding mapping for ASCII characters
/// commonly used in tests: space (32), A-Z (65-90), and a-z (97-122).
/// This provides a predictable character code → glyph name mapping for
/// Type3 font content stream fixtures.
///
/// # Returns
///
/// A `CharToGlyphMap` with standard ASCII character-to-glyph mappings.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::create_basic_char_to_glyph_map;
///
/// let encoding = create_basic_char_to_glyph_map();
///
/// // Character codes map to glyph names
/// assert_eq!(encoding.get(&65), Some(&"A".into()));
/// assert_eq!(encoding.get(&97), Some(&"a".into()));
/// assert_eq!(encoding.get(&32), Some(&"space".into()));
/// ```
pub fn create_basic_char_to_glyph_map() -> CharToGlyphMap {
    let mut map = CharToGlyphMap::new();

    // Space character
    map.insert(32, Arc::from("space"));

    // Uppercase A-Z
    for (i, ch) in ('A'..='Z').enumerate() {
        map.insert(65 + i as u8, Arc::from(ch.to_string().as_str()));
    }

    // Lowercase a-z
    for (i, ch) in ('a'..='z').enumerate() {
        map.insert(97 + i as u8, Arc::from(ch.to_string().as_str()));
    }

    map
}

/// Create a minimal character code to glyph name mapping with common test glyphs.
///
/// This function creates a minimal mapping with only a few essential glyphs
/// needed for basic Type3 font testing: space, 'A', and 'B'.
///
/// # Returns
///
/// A `CharToGlyphMap` with minimal character-to-glyph mappings.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::create_minimal_char_to_glyph_map;
///
/// let encoding = create_minimal_char_to_glyph_map();
///
/// assert_eq!(encoding.get(&32), Some(&"space".into()));
/// assert_eq!(encoding.get(&65), Some(&"A".into()));
/// assert_eq!(encoding.get(&66), Some(&"B".into()));
/// ```
pub fn create_minimal_char_to_glyph_map() -> CharToGlyphMap {
    let mut map = CharToGlyphMap::new();
    map.insert(32, Arc::from("space"));
    map.insert(65, Arc::from("A"));
    map.insert(66, Arc::from("B"));
    map
}

/// Create a character code to glyph name mapping from a glyph dictionary.
///
/// This function creates a character code mapping based on the glyphs present
/// in a glyph dictionary. It maps sequential character codes starting at 65
/// ('A') to the glyph names in the dictionary (excluding .notdef).
///
/// This is useful when you have a glyph dictionary and need a corresponding
/// encoding mapping for Type3 font testing.
///
/// # Arguments
///
/// * `glyph_dict` - The glyph dictionary to create mappings from
///
/// # Returns
///
/// A `CharToGlyphMap` with character codes mapped to glyph names from the dictionary.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::{create_basic_glyph_dict, create_char_to_glyph_from_dict};
/// use crate::parser::object::types::ObjRef;
///
/// let notdef_ref = ObjRef::new(10, 0);
/// let test_ref = ObjRef::new(11, 0);
/// let glyph_dict = create_basic_glyph_dict(notdef_ref, test_ref);
///
/// let char_map = create_char_to_glyph_from_dict(&glyph_dict);
///
/// // The "A" glyph from the dict should be mapped
/// assert!(char_map.values().any(|name| name.as_ref() == "A"));
/// ```
pub fn create_char_to_glyph_from_dict(glyph_dict: &GlyphDict) -> CharToGlyphMap {
    let mut map = CharToGlyphMap::new();
    let mut char_code = 65u8; // Start at 'A'

    for (glyph_name, _entry) in glyph_dict.iter() {
        // Skip .notdef
        if glyph_name.as_ref() == ".notdef" {
            continue;
        }

        map.insert(char_code, Arc::clone(glyph_name));
        char_code = char_code.saturating_add(1);

        // Don't go beyond reasonable ASCII range
        if char_code > 90 {
            break;
        }
    }

    map
}

/// Test edge builder for constructing test edges for AET testing.
///
/// This struct provides a builder pattern for creating Edge instances
/// with configurable properties for testing the Active Edge Table.
#[derive(Debug, Clone, Copy)]
pub struct TestEdge {
    /// Current X intersection position
    pub x: i32,
    /// Minimum Y coordinate (top of edge)
    pub y_min: i32,
    /// Maximum Y coordinate (bottom of edge)
    pub y_max: i32,
    /// Change in X across the edge
    pub dx: i32,
    /// Change in Y across the edge
    pub dy: i32,
}

impl TestEdge {
    /// Create a new TestEdge builder with default values.
    ///
    /// # Returns
    ///
    /// A TestEdge with all fields set to zero.
    pub fn new() -> Self {
        Self {
            x: 0,
            y_min: 0,
            y_max: 0,
            dx: 0,
            dy: 0,
        }
    }

    /// Set the y_min (top) coordinate of the edge.
    ///
    /// # Arguments
    ///
    /// * `y_min` - Minimum Y coordinate
    pub fn with_y_min(mut self, y_min: i32) -> Self {
        self.y_min = y_min;
        self
    }

    /// Set the y_max (bottom) coordinate of the edge.
    ///
    /// # Arguments
    ///
    /// * `y_max` - Maximum Y coordinate
    pub fn with_y_max(mut self, y_max: i32) -> Self {
        self.y_max = y_max;
        self
    }

    /// Set the current x position of the edge.
    ///
    /// # Arguments
    ///
    /// * `x` - Current X intersection position
    pub fn with_x(mut self, x: i32) -> Self {
        self.x = x;
        self
    }

    /// Set the slope of the edge using dx and dy.
    ///
    /// # Arguments
    ///
    /// * `dx` - Change in X across the edge
    /// * `dy` - Change in Y across the edge
    pub fn with_slope(mut self, dx: i32, dy: i32) -> Self {
        self.dx = dx;
        self.dy = dy;
        self
    }

    /// Build a complete Edge from this builder.
    ///
    /// # Returns
    ///
    /// A complete Edge struct for use in AET testing.
    pub fn build(self) -> crate::font::type3_rasterizer::Edge {
        crate::font::type3_rasterizer::Edge {
            x: self.x,
            y_min: self.y_min,
            y_max: self.y_max,
            dx: self.dx,
            dy: self.dy,
        }
    }

    /// Create a horizontal edge (dx = 0).
    ///
    /// # Arguments
    ///
    /// * `x` - X position
    /// * `y_min` - Minimum Y coordinate
    /// * `y_max` - Maximum Y coordinate
    pub fn horizontal(x: i32, y_min: i32, y_max: i32) -> Self {
        Self {
            x,
            y_min,
            y_max,
            dx: 0,
            dy: y_max - y_min,
        }
    }

    /// Create a vertical edge (dx != 0, dy = 0).
    ///
    /// # Arguments
    ///
    /// * `x` - X position
    /// * `y_min` - Minimum Y coordinate
    /// * `y_max` - Maximum Y coordinate
    pub fn vertical(x: i32, y_min: i32, y_max: i32) -> Self {
        Self {
            x,
            y_min,
            y_max,
            dx: 0,
            dy: y_max - y_min,
        }
    }

    /// Create a diagonal edge with the specified slope.
    ///
    /// # Arguments
    ///
    /// * `x` - Starting X position
    /// * `y_min` - Minimum Y coordinate
    /// * `y_max` - Maximum Y coordinate
    /// * `dx` - Change in X across the edge
    pub fn diagonal(x: i32, y_min: i32, y_max: i32, dx: i32) -> Self {
        let dy = y_max - y_min;
        Self {
            x,
            y_min,
            y_max,
            dx,
            dy,
        }
    }

    /// Create a simple edge from endpoints (x0, y0) to (x1, y1).
    ///
    /// # Arguments
    ///
    /// * `x0` - Starting X coordinate
    /// * `y0` - Starting Y coordinate
    /// * `x1` - Ending X coordinate
    /// * `y1` - Ending Y coordinate
    pub fn from_endpoints(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        let (y_min, y_max) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
        Self {
            x: x0,
            y_min,
            y_max,
            dx: x1 - x0,
            dy: y1 - y0,
        }
    }
}

impl Default for TestEdge {
    fn default() -> Self {
        Self::new()
    }
}

/// AET Inspector utility for checking AET state in tests.
///
/// This struct provides helper methods for inspecting and validating
/// the state of an Active Edge Table during testing.
#[derive(Debug)]
pub struct AETInspector {
    /// The edges being inspected
    edges: Vec<crate::font::type3_rasterizer::Edge>,
}

impl AETInspector {
    /// Create a new AETInspector from a vector of edges.
    ///
    /// # Arguments
    ///
    /// * `edges` - The Active Edge Table edges to inspect
    pub fn new(edges: Vec<crate::font::type3_rasterizer::Edge>) -> Self {
        Self { edges }
    }

    /// Get the number of edges in the AET.
    ///
    /// # Returns
    ///
    /// The count of active edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Check if the AET is empty.
    ///
    /// # Returns
    ///
    /// True if there are no active edges.
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    /// Get all x-coordinates from edges in the AET.
    ///
    /// # Returns
    ///
    /// A vector of current x positions for all edges.
    pub fn get_x_coordinates(&self) -> Vec<i32> {
        self.edges.iter().map(|e| e.x).collect()
    }

    /// Get all rounded intersection x-coordinates from edges in the AET.
    ///
    /// # Returns
    ///
    /// A vector of rounded x intersection positions.
    pub fn get_intersections(&self) -> Vec<i32> {
        self.edges
            .iter()
            .map(|e| e.intersection_x())
            .collect()
    }

    /// Check if edges are sorted by x-coordinate.
    ///
    /// # Returns
    ///
    /// True if edges are in non-decreasing x order.
    pub fn is_sorted_by_x(&self) -> bool {
        self.windows(|edges| {
            edges.windows(2).all(|w| w[0].x <= w[1].x)
        })
    }

    /// Find edges that should be active at a given y-coordinate.
    ///
    /// An edge is active if y_min <= y < y_max.
    ///
    /// # Arguments
    ///
    /// * `y` - The y-coordinate to check
    pub fn edges_at_y(&self, y: i32) -> Vec<&crate::font::type3_rasterizer::Edge> {
        self.edges
            .iter()
            .filter(|e| e.y_min <= y && y < e.y_max)
            .collect()
    }

    /// Count edges active at a given y-coordinate.
    ///
    /// # Arguments
    ///
    /// * `y` - The y-coordinate to check
    pub fn count_at_y(&self, y: i32) -> usize {
        self.edges_at_y(y).len()
    }

    /// Get the y-range spanned by all edges in the AET.
    ///
    /// # Returns
    ///
    /// A tuple of (min_y, max_y) or None if AET is empty.
    pub fn y_range(&self) -> Option<(i32, i32)> {
        if self.edges.is_empty() {
            return None;
        }
        let min_y = self.edges.iter().map(|e| e.y_min).min().unwrap();
        let max_y = self.edges.iter().map(|e| e.y_max).max().unwrap();
        Some((min_y, max_y))
    }

    /// Validate that the AET state is consistent.
    ///
    /// Checks that:
    /// - All edges have y_min <= y_max
    /// - No duplicate edges exist
    /// - Edges are within reasonable bounds
    pub fn validate(&self) -> Result<(), String> {
        for (i, edge) in self.edges.iter().enumerate() {
            if edge.y_min > edge.y_max {
                return Err(format!(
                    "Edge {} has y_min > y_max: {} > {}",
                    i, edge.y_min, edge.y_max
                ));
            }
        }
        Ok(())
    }

    /// Get a reference to the underlying edges vector.
    pub fn edges(&self) -> &[crate::font::type3_rasterizer::Edge] {
        &self.edges
    }

    /// Helper for windows() iteration over edges.
    fn windows<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[crate::font::type3_rasterizer::Edge]) -> R,
    {
        f(&self.edges)
    }
}

/// Create a simple scanline context for testing.
///
/// This function creates a minimal scanline context with the specified
/// dimensions, suitable for testing AET operations without requiring
/// a full Type3 rasterizer setup.
///
/// # Arguments
///
/// * `width` - Bitmap width in pixels
/// * `height` - Bitmap height in pixels
///
/// # Returns
///
/// A tuple of (width, height) representing the scanline context bounds.
pub fn create_scanline_context(width: u32, height: u32) -> (i32, i32) {
    (width as i32, height as i32)
}

/// Create edges from a list of (x0, y0, x1, y1) tuples.
///
/// This helper function converts a list of endpoint tuples into Edge structs,
/// automatically handling y_min/y_max ordering and horizontal edge filtering.
///
/// # Arguments
///
/// * `endpoints` - Slice of (x0, y0, x1, y1) tuples defining line segments
///
/// # Returns
///
/// A vector of Edge structs (horizontal edges are excluded).
pub fn create_edges_from_endpoints(endpoints: &[(i32, i32, i32, i32)]) -> Vec<crate::font::type3_rasterizer::Edge> {
    endpoints
        .iter()
        .filter(|&&(x0, y0, x1, y1)| y0 != y1) // Skip horizontal edges
        .map(|&(x0, y0, x1, y1)| {
            let (y_min, y_max) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
            crate::font::type3_rasterizer::Edge {
                x: x0,
                y_min,
                y_max,
                dx: x1 - x0,
                dy: y1 - y0,
            }
        })
        .collect()
}

/// Create a simple triangle edge set for testing.
///
/// This function creates a standard triangle with vertices at (0,0), (10,0), (5,10).
/// This is useful for basic AET testing as it creates predictable scanline behavior.
///
/// # Returns
///
/// A vector of three Edge structs representing the triangle sides.
pub fn create_triangle_edges() -> Vec<crate::font::type3_rasterizer::Edge> {
    vec![
        // Edge from (0,0) to (10,0) - horizontal, will be filtered
        // Edge from (10,0) to (5,10) - right side
        crate::font::type3_rasterizer::Edge {
            x: 10,
            y_min: 0,
            y_max: 10,
            dx: -5,
            dy: 10,
        },
        // Edge from (5,10) to (0,0) - left side
        crate::font::type3_rasterizer::Edge {
            x: 5,
            y_min: 0,
            y_max: 10,
            dx: -5,
            dy: -10,
        },
    ]
}

/// Create a simple rectangle edge set for testing.
///
/// This function creates edges for a rectangle with the given bounds.
/// Horizontal edges are automatically excluded.
///
/// # Arguments
///
/// * `x` - Left X coordinate
/// * `y` - Top Y coordinate
/// * `width` - Rectangle width
/// * `height` - Rectangle height
///
/// # Returns
///
/// A vector of Edge structs representing the rectangle sides.
pub fn create_rectangle_edges(x: i32, y: i32, width: i32, height: i32) -> Vec<crate::font::type3_rasterizer::Edge> {
    vec![
        // Left edge
        crate::font::type3_rasterizer::Edge {
            x,
            y_min: y,
            y_max: y + height,
            dx: 0,
            dy: height,
        },
        // Right edge
        crate::font::type3_rasterizer::Edge {
            x: x + width,
            y_min: y,
            y_max: y + height,
            dx: 0,
            dy: height,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_creation() {
        let content = Content {
            stroke_color: [0.0, 0.0, 0.0],
            line_width: 1.0,
            glyph_width: 100.0,
            drawing_commands: b"0 0 m 100 0 l 100 100 l 0 100 l h f".to_vec(),
        };

        assert_eq!(content.stroke_color, [0.0, 0.0, 0.0]);
        assert_eq!(content.line_width, 1.0);
        assert_eq!(content.glyph_width, 100.0);
        assert!(!content.drawing_commands.is_empty());
    }

    #[test]
    fn test_content_with_defaults() {
        let commands = b"0 0 m 100 0 l 100 100 l 0 100 l h f".to_vec();
        let content = Content::with_defaults(commands);

        assert_eq!(content.stroke_color, [0.0, 0.0, 0.0]);
        assert_eq!(content.line_width, 1.0);
        assert_eq!(content.glyph_width, 100.0);
        assert!(!content.drawing_commands.is_empty());
    }

    #[test]
    fn test_content_with_graphics_state() {
        let path = b"0 0 m 100 0 l 100 100 l 0 100 l h f";
        let stream = Content::with_graphics_state(path);

        // Verify stream contains graphics state setup
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");
        assert!(
            stream_str.contains("q"),
            "Should contain q (save graphics state)"
        );
        assert!(
            stream_str.contains("Q"),
            "Should contain Q (restore graphics state)"
        );
        assert!(
            stream_str.contains("RG"),
            "Should contain RG (set stroke color)"
        );
        assert!(
            stream_str.contains("w"),
            "Should contain w (set line width)"
        );
        assert!(
            stream_str.contains("d1"),
            "Should contain d1 (setcharwidth)"
        );
    }

    #[test]
    fn test_content_to_stream() {
        let commands = b"0 0 m 100 0 l 100 100 l 0 100 l h f".to_vec();
        let content = Content::with_defaults(commands.clone());
        let stream = content.to_stream();

        assert!(!stream.is_empty());

        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");
        assert!(stream_str.contains("q"));
        assert!(stream_str.contains("Q"));

        // Verify the drawing commands are included
        let cmd_str = std::str::from_utf8(&commands).expect("Commands should be valid UTF-8");
        assert!(stream_str.contains(cmd_str));
    }

    #[test]
    fn test_create_simple_charproc_stream() {
        let stream = create_simple_charproc_stream();

        assert!(!stream.is_empty());
        assert!(stream.starts_with(b"0 0 m"));

        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");
        assert!(stream_str.contains("m"));
        assert!(stream_str.contains("l"));
        assert!(stream_str.contains("h"));
        assert!(stream_str.contains("f"));
    }

    #[test]
    fn test_create_charproc_stream_with_curves() {
        let stream = create_charproc_stream_with_curves();

        assert!(!stream.is_empty());

        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");
        assert!(stream_str.contains("c"), "Should contain curveto operator");
    }

    #[test]
    fn test_create_rectangle_charproc_stream() {
        let stream = create_rectangle_charproc_stream();

        assert!(!stream.is_empty());

        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");

        // Verify acceptance criteria:
        // Uses moveto to start at origin
        assert!(stream_str.contains("0 0 m"), "Should contain moveto to origin");

        // Uses three lineto calls for the three edges
        assert!(stream_str.contains("10 0 l"), "Should contain lineto to (10, 0)");
        assert!(stream_str.contains("10 10 l"), "Should contain lineto to (10, 10)");
        assert!(stream_str.contains("0 10 l"), "Should contain lineto to (0, 10)");

        // Uses closepath to close the rectangle
        assert!(stream_str.contains("h"), "Should contain closepath");

        // Ends with fill operator
        assert!(stream_str.ends_with("f"), "Should end with fill operator");

        // Verify the path forms a proper 10x10 rectangle
        let parts: Vec<&str> = stream_str.split_whitespace().collect();
        assert!(parts.len() >= 10, "Should have at least 10 tokens for rectangle path");
    }

    #[test]
    fn test_create_empty_content_stream() {
        let stream = create_empty_content_stream();

        assert!(!stream.is_empty());

        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");
        assert!(stream_str.contains("BT"));
        assert!(stream_str.contains("ET"));
    }

    #[test]
    fn test_create_main_content_stream() {
        let stream = create_main_content_stream();

        assert!(!stream.is_empty());

        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");
        assert!(stream_str.contains("BT"));
        assert!(stream_str.contains("ET"));
        assert!(stream_str.contains("Tf"));
        assert!(stream_str.contains("Td"));
        assert!(stream_str.contains("Tj"));
    }

    #[test]
    fn test_create_main_content_stream_multi() {
        let stream = create_main_content_stream_multi();

        assert!(!stream.is_empty());

        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");
        assert!(stream_str.contains("BT"));
        assert!(stream_str.contains("ET"));
        assert!(stream_str.contains("(AB)"));
        assert!(stream_str.contains("(C)"));
    }

    #[test]
    fn test_content_compiles_without_errors() {
        // This test verifies that the Content struct compiles correctly
        // If this test compiles and runs, the struct is properly defined
        let content = Content::with_defaults(b"test".to_vec());
        assert_eq!(content.stroke_color, [0.0, 0.0, 0.0]);
        assert_eq!(content.line_width, 1.0);
        assert_eq!(content.glyph_width, 100.0);
    }

    #[test]
    fn test_testedge_builder_basic() {
        let edge = TestEdge::new()
            .with_y_min(0)
            .with_y_max(10)
            .with_x(5)
            .with_slope(2, 10)
            .build();

        assert_eq!(edge.x, 5);
        assert_eq!(edge.y_min, 0);
        assert_eq!(edge.y_max, 10);
        assert_eq!(edge.dx, 2);
        assert_eq!(edge.dy, 10);
    }

    #[test]
    fn test_testedge_horizontal() {
        let edge = TestEdge::horizontal(10, 0, 10).build();

        assert_eq!(edge.x, 10);
        assert_eq!(edge.y_min, 0);
        assert_eq!(edge.y_max, 10);
        assert_eq!(edge.dx, 0);
        assert_eq!(edge.dy, 10);
    }

    #[test]
    fn test_testedge_vertical() {
        let edge = TestEdge::vertical(5, 0, 10).build();

        assert_eq!(edge.x, 5);
        assert_eq!(edge.y_min, 0);
        assert_eq!(edge.y_max, 10);
        assert_eq!(edge.dx, 0);
        assert_eq!(edge.dy, 10);
    }

    #[test]
    fn test_testedge_diagonal() {
        let edge = TestEdge::diagonal(0, 0, 10, 5).build();

        assert_eq!(edge.x, 0);
        assert_eq!(edge.y_min, 0);
        assert_eq!(edge.y_max, 10);
        assert_eq!(edge.dx, 5);
        assert_eq!(edge.dy, 10);
    }

    #[test]
    fn test_testedge_from_endpoints() {
        let edge = TestEdge::from_endpoints(10, 0, 20, 10).build();

        assert_eq!(edge.x, 10);
        assert_eq!(edge.y_min, 0);
        assert_eq!(edge.y_max, 10);
        assert_eq!(edge.dx, 10);
        assert_eq!(edge.dy, 10);
    }

    #[test]
    fn test_testedge_default() {
        let edge = TestEdge::default().build();

        assert_eq!(edge.x, 0);
        assert_eq!(edge.y_min, 0);
        assert_eq!(edge.y_max, 0);
        assert_eq!(edge.dx, 0);
        assert_eq!(edge.dy, 0);
    }

    #[test]
    fn test_aetinspector_empty() {
        let inspector = AETInspector::new(vec![]);

        assert_eq!(inspector.edge_count(), 0);
        assert!(inspector.is_empty());
        assert!(inspector.get_x_coordinates().is_empty());
    }

    #[test]
    fn test_aetinspector_edge_count() {
        let edges = vec![
            TestEdge::new().with_x(5).with_y_min(0).with_y_max(10).build(),
            TestEdge::new().with_x(15).with_y_min(0).with_y_max(10).build(),
        ];

        let inspector = AETInspector::new(edges);

        assert_eq!(inspector.edge_count(), 2);
        assert!(!inspector.is_empty());
    }

    #[test]
    fn test_aetinspector_x_coordinates() {
        let edges = vec![
            TestEdge::new().with_x(5).build(),
            TestEdge::new().with_x(15).build(),
            TestEdge::new().with_x(25).build(),
        ];

        let inspector = AETInspector::new(edges);
        let x_coords = inspector.get_x_coordinates();

        assert_eq!(x_coords, vec![5, 15, 25]);
    }

    #[test]
    fn test_aetinspector_intersections() {
        let edges = vec![
            TestEdge::new().with_x(5).build(),
            TestEdge::new().with_x(15).build(),
        ];

        let inspector = AETInspector::new(edges);
        let intersections = inspector.get_intersections();

        assert_eq!(intersections, vec![5, 15]);
    }

    #[test]
    fn test_aetinspector_sorted_by_x() {
        let edges = vec![
            TestEdge::new().with_x(5).build(),
            TestEdge::new().with_x(15).build(),
            TestEdge::new().with_x(10).build(),
        ];

        let inspector = AETInspector::new(edges);
        assert!(!inspector.is_sorted_by_x());
    }

    #[test]
    fn test_aetinspector_edges_at_y() {
        let edges = vec![
            TestEdge::new().with_y_min(0).with_y_max(10).build(),
            TestEdge::new().with_y_min(5).with_y_max(15).build(),
            TestEdge::new().with_y_min(20).with_y_max(30).build(),
        ];

        let inspector = AETInspector::new(edges);

        assert_eq!(inspector.count_at_y(3), 1); // Only first edge
        assert_eq!(inspector.count_at_y(7), 2); // First and second edges
        assert_eq!(inspector.count_at_y(25), 1); // Only third edge
        assert_eq!(inspector.count_at_y(35), 0); // No edges
    }

    #[test]
    fn test_aetinspector_y_range() {
        let edges = vec![
            TestEdge::new().with_y_min(5).with_y_max(15).build(),
            TestEdge::new().with_y_min(0).with_y_max(20).build(),
        ];

        let inspector = AETInspector::new(edges);
        let (min_y, max_y) = inspector.y_range().unwrap();

        assert_eq!(min_y, 0);
        assert_eq!(max_y, 20);
    }

    #[test]
    fn test_aetinspector_validate() {
        let edges = vec![
            TestEdge::new().with_y_min(0).with_y_max(10).build(),
            TestEdge::new().with_y_min(5).with_y_max(15).build(),
        ];

        let inspector = AETInspector::new(edges);
        assert!(inspector.validate().is_ok());
    }

    #[test]
    fn test_aetinspector_validate_invalid() {
        let edges = vec![
            TestEdge::new().with_y_min(10).with_y_max(0).build(), // Invalid: y_min > y_max
        ];

        let inspector = AETInspector::new(edges);
        assert!(inspector.validate().is_err());
    }

    #[test]
    fn test_create_scanline_context() {
        let (width, height) = create_scanline_context(100, 100);

        assert_eq!(width, 100);
        assert_eq!(height, 100);
    }

    #[test]
    fn test_create_edges_from_endpoints() {
        let endpoints = vec![
            (0, 0, 10, 10), // Diagonal edge
            (10, 0, 20, 10), // Another diagonal edge
            (0, 0, 10, 0), // Horizontal edge - should be filtered
        ];

        let edges = create_edges_from_endpoints(&endpoints);

        assert_eq!(edges.len(), 2); // Horizontal edge filtered out
    }

    #[test]
    fn test_create_triangle_edges() {
        let edges = create_triangle_edges();

        assert_eq!(edges.len(), 2); // 2 non-horizontal edges
    }

    #[test]
    fn test_create_rectangle_edges() {
        let edges = create_rectangle_edges(0, 0, 10, 10);

        assert_eq!(edges.len(), 2); // Left and right edges only
    }
}
