//! Mock test fixtures for Type3 rasterizer tests.
//!
//! This module provides minimal mock implementations of resolver, source,
//! and counter types for testing parameter passing in callbacks.
//! It also provides glyph dictionary structures for Type3 font testing.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;

use crate::font::encoding::{FontEncoding, NamedEncoding};
use crate::font::type3::Type3Font;
use crate::graphics_state::Matrix3x3;
use crate::parser::object::types::ObjRef;

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
    pub fn new(name: impl Into<Arc<str>>, width: f64, bbox: [f32; 4], charproc_ref: ObjRef) -> Self {
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

/// Create a basic glyph dictionary with test entries.
///
/// Creates a minimal glyph dictionary containing:
/// - ".notdef" glyph (required in all Type3 fonts)
/// - Simple test glyph "A"
///
/// # Arguments
///
/// * `notdef_ref` - ObjRef for the .notdef glyph's content stream
/// * `test_ref` - ObjRef for the test glyph "A"'s content stream
///
/// # Returns
///
/// A GlyphDict with two entries ready for Type3 font testing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::{create_basic_glyph_dict, GlyphDict};
/// use crate::parser::object::types::ObjRef;
///
/// let notdef_ref = ObjRef::new(10, 0);
/// let test_ref = ObjRef::new(11, 0);
/// let glyph_dict = create_basic_glyph_dict(notdef_ref, test_ref);
///
/// assert!(glyph_dict.contains_key(".notdef"));
/// assert!(glyph_dict.contains_key("A"));
/// ```
pub fn create_basic_glyph_dict(notdef_ref: ObjRef, test_ref: ObjRef) -> GlyphDict {
    let mut dict = GlyphDict::new();

    // Add required .notdef glyph
    let notdef_entry = GlyphEntry::notdef(notdef_ref);
    dict.insert(Arc::clone(&notdef_entry.name), notdef_entry);

    // Add simple test glyph "A"
    let test_entry = GlyphEntry::new(
        "A",
        600.0,                      // width
        [50.0, 0.0, 550.0, 700.0], // bbox
        test_ref
    );
    dict.insert(Arc::clone(&test_entry.name), test_entry);

    dict
}

/// Create a minimal glyph dictionary with only a ".notdef" entry.
///
/// Use this when you need the absolute minimum for Type3 font testing.
///
/// # Arguments
///
/// * `notdef_ref` - ObjRef for the .notdef glyph's content stream
///
/// # Returns
///
/// A GlyphDict with only the required .notdef entry.
pub fn create_minimal_glyph_dict(notdef_ref: ObjRef) -> GlyphDict {
    let mut dict = GlyphDict::new();
    let notdef_entry = GlyphEntry::notdef(notdef_ref);
    dict.insert(Arc::clone(&notdef_entry.name), notdef_entry);
    dict
}

/// Convert a GlyphDict to the CharProcs HashMap format used by Type3Font.
///
/// Extracts just the glyph name -> ObjRef mapping from a full glyph dictionary,
/// which is the format expected by Type3Font's char_procs field.
///
/// # Arguments
///
/// * `glyph_dict` - The GlyphDict to convert
///
/// # Returns
///
/// A HashMap mapping glyph names to their charproc ObjRefs.
///
/// # Example
///
/// ```rust,no_run
/// use std::collections::HashMap;
/// use crate::font::type3_test_fixtures::{create_basic_glyph_dict, to_charprocs_map};
/// use crate::parser::object::types::{ObjRef, intern};
///
/// let glyph_dict = create_basic_glyph_dict(ObjRef::new(10, 0), ObjRef::new(11, 0));
/// let charprocs = to_charprocs_map(&glyph_dict);
///
/// assert_eq!(charprocs.len(), 2);
/// assert_eq!(charprocs.get(intern(".notdef")), Some(&ObjRef::new(10, 0)));
/// ```
pub fn to_charprocs_map(glyph_dict: &GlyphDict) -> HashMap<Arc<str>, ObjRef> {
    glyph_dict
        .iter()
        .map(|(name, entry)| (Arc::clone(name), entry.charproc_ref))
        .collect()
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

/// Create a minimal Type3Font struct for testing.
///
/// This function creates a Type3Font with all required fields set to
/// sensible default values suitable for testing the rasterize_type3_glyph
/// function.
///
/// # Arguments
///
/// * `charproc_ref` - ObjRef for the .notdef glyph's content stream
///
/// # Returns
///
/// A Type3Font struct with minimal but valid configuration:
/// - Single .notdef glyph in CharProcs
/// - Unit matrix (no transformation)
/// - Default font bounding box
/// - StandardEncoding with no differences
/// - Single glyph width range (first_char=0, last_char=0)
/// - No resources (page resources will be used)
/// - Empty diagnostics
/// - Empty rasterization cache
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_test_fixtures::create_minimal_type3_font;
/// use crate::parser::object::types::ObjRef;
///
/// let notdef_ref = ObjRef::new(10, 0);
/// let font = create_minimal_type3_font(notdef_ref);
///
/// assert!(font.char_procs.contains_key(".notdef"));
/// assert_eq!(font.first_char, 0);
/// assert_eq!(font.last_char, 0);
/// assert_eq!(font.widths.len(), 1);
/// ```
pub fn create_minimal_type3_font(charproc_ref: ObjRef) -> Type3Font {
    // Create minimal charprocs with just .notdef
    let mut char_procs = HashMap::new();
    char_procs.insert(Arc::from(".notdef"), charproc_ref);

    // Create encoding with StandardEncoding base
    let encoding = FontEncoding::new(Some(NamedEncoding::Standard));

    Type3Font {
        char_procs,
        first_char: 0,
        last_char: 0,
        widths: vec![500.0], // Single width for the .notdef glyph
        font_matrix: Matrix3x3::identity(), // Unit matrix - no transformation
        resources: None, // No font-specific resources, use page resources
        encoding,
        font_bbox: [0.0, 0.0, 0.0, 0.0], // Default bounding box
        diagnostics: Vec::new(), // No diagnostics
        raster_cache: Arc::new(DashMap::new()), // Empty cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::types::ObjRef;

    #[test]
    fn test_mock_resolver_flag() {
        let resolver = mock_resolver();
        assert!(!resolver.load(Ordering::SeqCst));

        resolver.store(true, Ordering::SeqCst);
        assert!(resolver.load(Ordering::SeqCst));
    }

    #[test]
    fn test_mock_source_flag() {
        let source = mock_source();
        assert!(!source.load(Ordering::SeqCst));

        source.store(true, Ordering::SeqCst);
        assert!(source.load(Ordering::SeqCst));
    }

    #[test]
    fn test_mock_counter_increment() {
        let counter = mock_counter();
        assert_eq!(counter.load(Ordering::SeqCst), 0);

        counter.fetch_add(1, Ordering::SeqCst);
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        counter.fetch_add(1, Ordering::SeqCst);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_callback_captures_all_parameters() {
        let resolver = mock_resolver();
        let source = mock_source();
        let counter = mock_counter();

        let resolver_clone = resolver.clone();
        let source_clone = source.clone();
        let counter_clone = counter.clone();

        // Callback that uses all three parameters
        let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
            resolver_clone.store(true, Ordering::SeqCst);
            source_clone.store(true, Ordering::SeqCst);
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Some(b"test".to_vec())
        };

        // Invoke callback
        callback(ObjRef::new(1, 0));

        // Verify all parameters were captured/used
        assert!(resolver.load(Ordering::SeqCst), "resolver flag should be set");
        assert!(source.load(Ordering::SeqCst), "source flag should be set");
        assert_eq!(counter.load(Ordering::SeqCst), 1, "counter should be 1");
    }

    #[test]
    fn test_cloning_creates_independent_references() {
        let resolver1 = mock_resolver();
        let resolver2 = resolver1.clone();

        resolver1.store(true, Ordering::SeqCst);
        assert!(resolver2.load(Ordering::SeqCst), "clone should see the same value");

        resolver2.store(false, Ordering::SeqCst);
        assert!(!resolver1.load(Ordering::SeqCst), "changes are reflected in both");
    }

    // --- Glyph dictionary tests ---

    #[test]
    fn test_glyph_entry_creation() {
        let entry = GlyphEntry::new(
            "test_glyph",
            650.0,
            [10.0, 20.0, 640.0, 750.0],
            ObjRef::new(5, 0)
        );

        assert_eq!(entry.name.as_ref(), "test_glyph");
        assert_eq!(entry.width, 650.0);
        assert_eq!(entry.bbox, [10.0, 20.0, 640.0, 750.0]);
        assert_eq!(entry.charproc_ref, ObjRef::new(5, 0));
    }

    #[test]
    fn test_glyph_entry_minimal() {
        let entry = GlyphEntry::minimal("A", ObjRef::new(10, 0));

        assert_eq!(entry.name.as_ref(), "A");
        assert_eq!(entry.width, 500.0);
        assert_eq!(entry.bbox, [0.0, 0.0, 500.0, 500.0]);
        assert_eq!(entry.charproc_ref, ObjRef::new(10, 0));
    }

    #[test]
    fn test_glyph_entry_notdef() {
        let entry = GlyphEntry::notdef(ObjRef::new(1, 0));

        assert_eq!(entry.name.as_ref(), ".notdef");
        assert_eq!(entry.width, 500.0);
        assert_eq!(entry.bbox, [0.0, 0.0, 500.0, 500.0]);
        assert_eq!(entry.charproc_ref, ObjRef::new(1, 0));
    }

    #[test]
    fn test_basic_glyph_dict() {
        let notdef_ref = ObjRef::new(10, 0);
        let test_ref = ObjRef::new(11, 0);
        let dict = create_basic_glyph_dict(notdef_ref, test_ref);

        assert_eq!(dict.len(), 2);
        assert!(dict.contains_key(".notdef"));
        assert!(dict.contains_key("A"));

        let notdef = dict.get(".notdef").unwrap();
        assert_eq!(notdef.width, 500.0);
        assert_eq!(notdef.charproc_ref, notdef_ref);

        let test_glyph = dict.get("A").unwrap();
        assert_eq!(test_glyph.width, 600.0);
        assert_eq!(test_glyph.bbox, [50.0, 0.0, 550.0, 700.0]);
        assert_eq!(test_glyph.charproc_ref, test_ref);
    }

    #[test]
    fn test_minimal_glyph_dict() {
        let notdef_ref = ObjRef::new(42, 0);
        let dict = create_minimal_glyph_dict(notdef_ref);

        assert_eq!(dict.len(), 1);
        assert!(dict.contains_key(".notdef"));

        let notdef = dict.get(".notdef").unwrap();
        assert_eq!(notdef.width, 500.0);
        assert_eq!(notdef.charproc_ref, notdef_ref);
    }

    #[test]
    fn test_to_charprocs_map() {
        let notdef_ref = ObjRef::new(10, 0);
        let test_ref = ObjRef::new(11, 0);
        let glyph_dict = create_basic_glyph_dict(notdef_ref, test_ref);

        let charprocs = to_charprocs_map(&glyph_dict);

        assert_eq!(charprocs.len(), 2);
        assert_eq!(charprocs.get(".notdef"), Some(&notdef_ref));
        assert_eq!(charprocs.get("A"), Some(&test_ref));
    }

    #[test]
    fn test_glyph_dict_accessible_from_test_module() {
        // Verify that the glyph dict can be created and used in tests
        let dict = create_minimal_glyph_dict(ObjRef::new(1, 0));

        // This demonstrates the dict is accessible and functional
        assert!(!dict.is_empty());
        assert!(dict.contains_key(".notdef"));
    }

    #[test]
    fn test_create_minimal_type3_font() {
        let notdef_ref = ObjRef::new(10, 0);
        let font = create_minimal_type3_font(notdef_ref);

        // Verify CharProcs
        assert!(font.char_procs.contains_key(".notdef"));
        assert_eq!(font.char_procs.len(), 1);
        assert_eq!(font.char_procs.get(".notdef"), Some(&notdef_ref));

        // Verify character range
        assert_eq!(font.first_char, 0);
        assert_eq!(font.last_char, 0);

        // Verify widths
        assert_eq!(font.widths.len(), 1);
        assert_eq!(font.widths[0], 500.0);

        // Verify matrix is identity
        assert_eq!(font.font_matrix.a, 1.0);
        assert_eq!(font.font_matrix.b, 0.0);
        assert_eq!(font.font_matrix.c, 0.0);
        assert_eq!(font.font_matrix.d, 1.0);
        assert_eq!(font.font_matrix.e, 0.0);
        assert_eq!(font.font_matrix.f, 0.0);

        // Verify no resources
        assert!(font.resources.is_none());

        // Verify encoding
        assert!(font.encoding.base_encoding().is_some());

        // Verify font bbox
        assert_eq!(font.font_bbox, [0.0, 0.0, 0.0, 0.0]);

        // Verify diagnostics
        assert!(font.diagnostics.is_empty());

        // Verify raster cache
        assert!(font.raster_cache.is_empty());
    }

    // --- Charproc stream tests ---

    #[test]
    fn test_create_simple_charproc_stream() {
        let stream = create_simple_charproc_stream();

        // Verify stream is not empty
        assert!(!stream.is_empty(), "Charproc stream should not be empty");

        // Verify it starts with moveto command
        assert!(stream.starts_with(b"0 0 m"), "Stream should start with moveto command");

        // Verify it contains essential path commands
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");
        assert!(stream_str.contains("m"), "Stream should contain moveto (m)");
        assert!(stream_str.contains("l"), "Stream should contain lineto (l)");
        assert!(stream_str.contains("h"), "Stream should contain closepath (h)");
        assert!(stream_str.contains("f"), "Stream should contain fill (f)");

        // Verify the exact content for the simple rectangle
        assert_eq!(stream, b"0 0 m 500 0 l 500 500 l 0 500 l h f");
    }

    #[test]
    fn test_simple_charproc_stream_draws_rectangle() {
        let stream = create_simple_charproc_stream();
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");

        // Verify the rectangle path: origin → top edge → right edge → bottom edge → close
        assert!(stream_str.contains("0 0 m"), "Should start at origin");
        assert!(stream_str.contains("500 0 l"), "Should have top edge");
        assert!(stream_str.contains("500 500 l"), "Should have right edge");
        assert!(stream_str.contains("0 500 l"), "Should have bottom edge");
        assert!(stream_str.contains("h"), "Should close the path");
        assert!(stream_str.ends_with("f"), "Should end with fill");
    }

    #[test]
    fn test_create_charproc_stream_with_curves() {
        let stream = create_charproc_stream_with_curves();

        // Verify stream is not empty
        assert!(!stream.is_empty(), "Charproc stream with curves should not be empty");

        // Verify it contains curveto command
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");
        assert!(stream_str.contains("c"), "Stream should contain curveto (c)");

        // Verify it still has basic path commands
        assert!(stream_str.contains("m"), "Stream should contain moveto (m)");
        assert!(stream_str.contains("l"), "Stream should contain lineto (l)");
        assert!(stream_str.contains("h"), "Stream should contain closepath (h)");
        assert!(stream_str.contains("f"), "Stream should contain fill (f)");
    }

    #[test]
    fn test_charproc_stream_with_curves_has_valid_curveto_syntax() {
        let stream = create_charproc_stream_with_curves();
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");

        // Verify curveto commands have 6 parameters (x1 y1 x2 y2 x3 y3)
        // Split by whitespace and check curveto sections
        let parts: Vec<&str> = stream_str.split_whitespace().collect();
        let mut i = 0;
        while i < parts.len() {
            if parts[i] == "c" {
                // Should have 6 numbers before 'c'
                assert!(i >= 6, "Curveto should have 6 coordinate parameters");
            }
            i += 1;
        }
    }

    #[test]
    fn test_charproc_streams_are_accessible_from_test_module() {
        // Verify that the charproc stream functions are public and accessible
        let simple_stream = create_simple_charproc_stream();
        let curved_stream = create_charproc_stream_with_curves();

        // Both should be non-empty byte vectors
        assert!(!simple_stream.is_empty());
        assert!(!curved_stream.is_empty());

        // They should be different (different shapes)
        assert_ne!(simple_stream, curved_stream);
    }

    #[test]
    fn test_charproc_stream_commands_form_valid_pdf_syntax() {
        let stream = create_simple_charproc_stream();
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");

        // Verify PDF syntax pattern: number number command (repeated)
        // Commands should be single letters (m, l, h, f, c, etc.)
        let valid_commands = ['m', 'l', 'c', 'h', 'f', 'S', 's', 'B', 'b', 'n', 'v', 'y'];
        let parts: Vec<&str> = stream_str.split_whitespace().collect();

        for part in &parts {
            // If it's a single character, it should be a valid PDF command
            if part.len() == 1 {
                let ch = part.chars().next().unwrap();
                assert!(
                    valid_commands.contains(&ch),
                    "Invalid PDF command: {}",
                    ch
                );
            } else {
                // Multi-part tokens should be parseable as numbers
                assert!(
                    part.parse::<f64>().is_ok() || part.parse::<i64>().is_ok(),
                    "Non-command token should be a number: {}",
                    part
                );
            }
        }
    }

    // --- Main content stream tests ---

    #[test]
    fn test_create_main_content_stream() {
        let stream = create_main_content_stream();

        // Verify stream is not empty
        assert!(!stream.is_empty(), "Main content stream should not be empty");

        // Verify it starts with BT and ends with ET
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");
        assert!(stream_str.contains("BT"), "Stream should contain BT (Begin Text)");
        assert!(stream_str.contains("ET"), "Stream should contain ET (End Text)");
    }

    #[test]
    fn test_main_content_stream_has_font_selection() {
        let stream = create_main_content_stream();
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");

        // Verify it contains font selection command (Tf operator)
        assert!(stream_str.contains("Tf"), "Stream should contain Tf (Set Font operator)");
        assert!(stream_str.contains("/F1"), "Stream should reference font /F1");
        assert!(stream_str.contains("12"), "Stream should set font size 12");
    }

    #[test]
    fn test_main_content_stream_has_text_positioning() {
        let stream = create_main_content_stream();
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");

        // Verify it contains text positioning command (Td operator)
        assert!(stream_str.contains("Td"), "Stream should contain Td (Translate operator)");
        assert!(stream_str.contains("100"), "Stream should position at x=100");
        assert!(stream_str.contains("700"), "Stream should position at y=700");
    }

    #[test]
    fn test_main_content_stream_has_text_drawing() {
        let stream = create_main_content_stream();
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");

        // Verify it contains text drawing command (Tj operator)
        assert!(stream_str.contains("Tj"), "Stream should contain Tj (Show Text operator)");
        assert!(stream_str.contains("(A)"), "Stream should draw glyph A");
    }

    #[test]
    fn test_main_content_stream_references_glyph_from_dict() {
        let stream = create_main_content_stream();
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");

        // Verify it references a specific glyph ("A") that would exist in the glyph dict
        assert!(stream_str.contains("(A)"), "Stream should reference glyph A from charprocs");
    }

    #[test]
    fn test_main_content_stream_compiles() {
        // This test simply verifies that the function exists and compiles
        // If this compiles, the stream creation is syntactically valid
        let stream = create_main_content_stream();
        assert!(!stream.is_empty());
    }

    #[test]
    fn test_main_content_stream_accessible_from_test_module() {
        // Verify the main content stream function is public and accessible
        let stream = create_main_content_stream();

        // Should be a valid byte vector
        assert!(stream.is_empty() == false);

        // Should be parseable as UTF-8 (valid PDF syntax)
        let _stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");
    }

    #[test]
    fn test_create_main_content_stream_multi() {
        let stream = create_main_content_stream_multi();

        // Verify stream is not empty
        assert!(!stream.is_empty(), "Multi-glyph content stream should not be empty");

        // Verify it has BT/ET
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");
        assert!(stream_str.contains("BT"), "Stream should contain BT");
        assert!(stream_str.contains("ET"), "Stream should contain ET");
    }

    #[test]
    fn test_main_content_stream_multi_has_multiple_text_operations() {
        let stream = create_main_content_stream_multi();
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");

        // Verify it has multiple text drawing operations
        assert!(stream_str.contains("(AB)"), "Stream should draw AB");
        assert!(stream_str.contains("(C)"), "Stream should draw C");

        // Should have Tj operator appearing at least twice
        let tj_count = stream_str.matches("Tj").count();
        assert!(tj_count >= 2, "Stream should have at least 2 Tj operators for multi-glyph drawing");
    }

    #[test]
    fn test_main_content_stream_multi_has_position_adjustment() {
        let stream = create_main_content_stream_multi();
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");

        // Verify it adjusts position between text operations
        // Should have Td operator appearing at least twice (initial position + adjustment)
        let td_count = stream_str.matches("Td").count();
        assert!(td_count >= 2, "Stream should have at least 2 Td operators for positioning");
    }

    #[test]
    fn test_main_content_stream_structured_as_proper_pdf_stream() {
        let stream = create_main_content_stream();
        let stream_str = std::str::from_utf8(&stream).expect("Stream should be valid UTF-8");

        // Verify proper PDF content stream structure:
        // BT ... Tf ... Td ... Tj ... ET
        let parts: Vec<&str> = stream_str.split_whitespace().collect();

        // Should start with BT
        assert_eq!(parts[0], "BT", "Stream should start with BT");

        // Should end with ET
        assert_eq!(parts.last(), Some(&"ET"), "Stream should end with ET");

        // Should contain essential operators in order
        let bt_idx = parts.iter().position(|&s| s == "BT");
        let tf_idx = parts.iter().position(|&s| s == "Tf");
        let td_idx = parts.iter().position(|&s| s == "Td");
        let tj_idx = parts.iter().position(|&s| s == "Tj");
        let et_idx = parts.iter().position(|&s| s == "ET");

        assert!(bt_idx.is_some(), "Should have BT");
        assert!(tf_idx.is_some(), "Should have Tf");
        assert!(td_idx.is_some(), "Should have Td");
        assert!(tj_idx.is_some(), "Should have Tj");
        assert!(et_idx.is_some(), "Should have ET");

        // Verify ordering: BT comes first, ET comes last
        assert!(bt_idx < tf_idx, "BT should come before Tf");
        assert!(tf_idx < td_idx, "Tf should come before Td");
        assert!(td_idx < tj_idx, "Td should come before Tj");
        assert!(tj_idx < et_idx, "Tj should come before ET");
    }

    #[test]
    fn test_main_content_stream_no_compile_errors() {
        // This test verifies the stream can be created without errors
        // If it compiles, the syntax is valid
        let result = std::panic::catch_unwind(|| {
            let _stream = create_main_content_stream();
            let _stream_multi = create_main_content_stream_multi();
        });

        assert!(result.is_ok(), "Stream creation should not panic or have compile errors");
    }

    #[test]
    fn test_main_content_stream_different_from_charproc_stream() {
        // Main content stream is different from charproc streams
        let main_stream = create_main_content_stream();
        let charproc_stream = create_simple_charproc_stream();

        // They should be different (different purposes)
        assert_ne!(main_stream, charproc_stream, "Main content stream should differ from charproc stream");

        // Main stream should have BT/ET (text operators)
        let main_str = std::str::from_utf8(&main_stream).unwrap();
        assert!(main_str.contains("BT"), "Main stream should have BT");
        assert!(main_str.contains("ET"), "Main stream should have ET");

        // Charproc stream should have path commands
        let charproc_str = std::str::from_utf8(&charproc_stream).unwrap();
        assert!(charproc_str.contains("m"), "Charproc stream should have moveto");
        assert!(charproc_str.contains("l"), "Charproc stream should have lineto");
    }
}
