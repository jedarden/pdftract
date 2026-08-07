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
}
