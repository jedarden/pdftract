//! Test fixtures module for font testing utilities.
//!
//! This module provides helper functions and fixtures for testing font-related
//! functionality across the pdftract crate. It aims to keep test helper code
//! organized and separate from production code.

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;

use crate::diagnostics::Diagnostic;
use crate::font::encoding::FontEncoding;
use crate::graphics_state::Matrix3x3;
use crate::parser::object::types::{ObjRef, PdfDict};

/// Placeholder function to verify compilation.
///
/// This function serves as a minimal verification that the test_fixtures module
/// is properly included and compiles successfully. It can be removed once actual
/// test fixtures are implemented.
pub fn _test_fixtures_loaded() -> bool {
    true
}

/// Creates a minimal valid FontEncoding for testing.
///
/// This function returns a FontEncoding with no base encoding set (None),
/// which is suitable for basic Type3Font construction in tests.
///
/// # Returns
///
/// A `FontEncoding` instance created with `FontEncoding::new(None)`.
///
/// # Use in Tests
///
/// ```rust
/// use pdftract_core::font::test_fixtures::font_encoding_default;
/// let encoding = font_encoding_default();
/// ```
pub fn font_encoding_default() -> FontEncoding {
    FontEncoding::new(None)
}

/// Creates an identity 3x3 transformation matrix for testing.
///
/// This function returns an identity matrix, which is the default
/// transformation matrix used in Type3Font construction when no specific
/// transformation is required.
///
/// # Returns
///
/// A `Matrix3x3` instance representing the identity transformation,
/// created with `Matrix3x3::identity()`.
///
/// # Use in Tests
///
/// ```rust
/// use pdftract_core::font::test_fixtures::matrix_identity;
/// let matrix = matrix_identity();
/// ```
pub fn matrix_identity() -> Matrix3x3 {
    Matrix3x3::identity()
}

/// Creates a minimal Type3Font for testing.
///
/// This function provides the foundation for Type3Font tests by constructing
/// a Type3Font instance with the simplest required fields set to sensible defaults.
///
/// # Returns
///
/// A `Type3Font` instance with:
/// - Empty char_procs HashMap
/// - first_char: 0, last_char: 0 (single glyph range)
/// - widths: vec\![100.0\] (single 100-unit width)
/// - font_bbox: [0.0, 0.0, 100.0, 100.0] (100x100 glyph space)
/// - encoding: default (no base encoding set)
/// - font_matrix: identity matrix
/// - resources: None
/// - diagnostics: empty Vec
/// - raster_cache: empty DashMap
///
/// # Use in Tests
///
/// ```rust
/// use pdftract_core::font::test_fixtures::type3_font_minimal;
/// let font = type3_font_minimal();
/// assert_eq!(font.first_char, 0);
/// assert_eq!(font.last_char, 0);
/// ```
pub fn type3_font_minimal() -> crate::font::type3::Type3Font {
    crate::font::type3::Type3Font {
        char_procs: HashMap::new(),
        first_char: 0,
        last_char: 0,
        widths: vec![100.0],
        font_bbox: [0.0, 0.0, 100.0, 100.0],
        encoding: font_encoding_default(),
        font_matrix: matrix_identity(),
        resources: None,
        diagnostics: Vec::new(),
        raster_cache: Arc::new(DashMap::new()),
    }
}

/// Creates a mock glyph dictionary (CharProcs) with basic properties.
///
/// This function creates a HashMap mapping glyph names to their object references,
/// which is the core structure needed for Type3 glyph rendering. The dictionary
/// contains a ".notdef" glyph (the standard PDF "missing glyph" symbol) as its
/// entry, which can be populated with actual content stream data in subsequent
/// test setup.
///
/// # Returns
///
/// A `HashMap<Arc<str>, ObjRef>` with:
/// - ".notdef" mapped to ObjRef(1, 0) (placeholder reference to glyph stream)
/// - Additional entries can be added as needed for specific test scenarios
///
/// # Use in Tests
///
/// ```rust
/// use pdftract_core::font::test_fixtures::mock_glyph_dict;
/// use std::sync::Arc;
///
/// let char_procs = mock_glyph_dict();
/// assert!(char_procs.contains_key(".notdef"));
/// assert_eq!(char_procs.len(), 1);
///
/// // Add more glyphs as needed
/// char_procs.insert(Arc::from("A"), ObjRef::new(2, 0));
/// ```
///
/// # Purpose
///
/// The glyph dictionary (CharProcs) maps character names to their drawing programs.
/// Without it, the rasterizer has nothing to render. The ".notdef" glyph is a
/// required glyph in PDF fonts that represents the "undefined character" - it's
/// what gets rendered when a glyph name is not found in the font.
pub fn mock_glyph_dict() -> HashMap<Arc<str>, ObjRef> {
    let mut char_procs = HashMap::new();
    // Add the standard ".notdef" glyph (PDF's "missing glyph" symbol)
    // ObjRef(1, 0) is a placeholder reference - the actual content stream
    // will be provided by the test resolver function
    char_procs.insert(Arc::from(".notdef"), ObjRef::new(1, 0));
    char_procs
}
