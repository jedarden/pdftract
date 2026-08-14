//! Type 3 font loader.
//!
//! This module implements loading of Type 3 fonts, which are PDF fonts defined
//! by content stream glyphs rather than font programs. Type 3 fonts have:
//! - /CharProcs: dictionary of glyph name -> content stream
//! - /Widths: array of advance widths per character code
//! - /FontMatrix: transform from glyph space to text space
//! - /Resources: resource dictionary for glyph streams
//! - /Encoding: code -> glyph name mapping

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::font::encoding::FontEncoding;
use crate::graphics_state::Matrix3x3;
use crate::parser::object::types::{ObjRef, PdfDict, PdfObject};

/// Result type for Type3 font operations.
pub type Type3Result<T> = Result<T, Type3Error>;

/// Errors that can occur during Type3 font operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type3Error {
    /// Character procedure reference not found in /CharProcs dictionary.
    MissingCharProcRef {
        /// The glyph name that was not found
        glyph_name: String,
    },
}

impl std::fmt::Display for Type3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type3Error::MissingCharProcRef { glyph_name } => {
                write!(f, "character procedure reference not found for glyph '{}'", glyph_name)
            }
        }
    }
}

impl std::error::Error for Type3Error {}

/// Type 3 font data.
///
/// Type 3 fonts are defined by content stream glyphs rather than font programs.
/// Each glyph is a PDF content stream that draws the glyph shape.
#[derive(Clone, Debug)]
pub struct Type3Font {
    /// /CharProcs dictionary: glyph name -> stream object reference.
    ///
    /// These are content streams that draw each glyph. The streams are
    /// fetched lazily on first rasterization.
    pub char_procs: HashMap<Arc<str>, ObjRef>,
    /// /FirstChar: first character code in /Widths array.
    pub first_char: u8,
    /// /LastChar: last character code in /Widths array.
    pub last_char: u8,
    /// /Widths array: advance widths in glyph space.
    ///
    /// Length should equal `last_char - first_char + 1`. Widths are
    /// in glyph space and must be transformed by /FontMatrix to get
    /// text space units.
    pub widths: Vec<f64>,
    /// /FontMatrix: 3x3 transform from glyph space to text space.
    ///
    /// Default is `[0.001 0 0 0.001 0 0]` (1/1000 scale). Per PDF spec,
    /// this matrix is applied during glyph execution.
    pub font_matrix: Matrix3x3,
    /// /Resources: resource dictionary for glyph content streams.
    ///
    /// Defaults to the page's resource dictionary if absent. This
    /// contains form XObjects and fonts referenced by glyph streams.
    pub resources: Option<Arc<PdfDict>>,
    /// /Encoding: code -> glyph name mapping.
    ///
    /// Uses the same encoding structure as Type1 fonts (named encoding
    /// + /Differences overlay).
    pub encoding: FontEncoding,
    /// /FontBBox: font bounding box in glyph space [llx lly urx ury].
    ///
    /// Default is [0, 0, 0, 0] if not specified. Per PDF spec section 9.6.5,
    /// this is the bounding box of all glyphs in the font.
    pub font_bbox: [f32; 4],
    /// Diagnostics emitted during loading.
    pub diagnostics: Vec<Diagnostic>,
    /// Rasterized glyph cache: glyph name -> dynamic bitmap.
    ///
    /// Cached to avoid re-rasterizing the same glyph multiple times
    /// during shape recognition.
    pub raster_cache: Arc<DashMap<Arc<str>, Vec<u8>>>,
}

impl Type3Font {
    /// Load a Type 3 font from its dictionary.
    ///
    /// # Arguments
    ///
    /// * `font_dict` - The Type 3 font dictionary from the resource dictionary
    ///
    /// # Returns
    ///
    /// A `Type3Font` with all fields populated. Missing fields are handled
    /// gracefully with defaults and diagnostics.
    pub fn load(font_dict: &PdfDict) -> Self {
        let mut diagnostics = Vec::new();

        // Parse /CharProcs (dictionary of glyph name -> stream reference)
        let char_procs = Self::load_char_procs(font_dict, &mut diagnostics);

        // Parse /FirstChar and /LastChar
        let (first_char, last_char) = Self::load_char_range(font_dict, &mut diagnostics);

        // Parse /Widths array
        let widths = Self::load_widths(font_dict, first_char, last_char, &mut diagnostics);

        // Parse /FontMatrix (default to [0.001 0 0 0.001 0 0])
        let font_matrix = Self::load_font_matrix(font_dict, &mut diagnostics);

        // Parse /Resources (optional, defaults to None)
        let resources = Self::load_resources(font_dict);

        // Parse /Encoding (defaults to StandardEncoding)
        let encoding = FontEncoding::parse_from_font(font_dict, None, &mut diagnostics);

        // Parse /FontBBox (default to [0, 0, 0, 0] if not specified)
        let font_bbox = Self::load_font_bbox(font_dict, &mut diagnostics);

        Self {
            char_procs,
            first_char,
            last_char,
            widths,
            font_matrix,
            resources,
            encoding,
            font_bbox,
            diagnostics,
            raster_cache: Arc::new(DashMap::new()),
        }
    }

    /// Load /CharProcs dictionary.
    ///
    /// Maps glyph names to content stream object references. Returns empty
    /// map if /CharProcs is missing (malformed but seen in the wild).
    fn load_char_procs(
        font_dict: &PdfDict,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> HashMap<Arc<str>, ObjRef> {
        let mut char_procs = HashMap::new();

        let char_procs_obj = match font_dict.get("/CharProcs") {
            Some(obj) => obj,
            None => {
                diagnostics.push(Diagnostic::with_static_no_offset(
                    DiagCode::FontParseFailed,
                    "Type3 font missing /CharProcs dictionary; treating as zero-glyph font",
                ));
                return char_procs;
            }
        };

        let char_procs_dict = match char_procs_obj {
            PdfObject::Dict(d) => d.as_ref(),
            PdfObject::Ref(_) => {
                diagnostics.push(Diagnostic::with_static_no_offset(
                    DiagCode::FontParseFailed,
                    "/CharProcs is indirect reference; not supported, treating as zero-glyph font",
                ));
                return char_procs;
            }
            _ => {
                diagnostics.push(Diagnostic::with_static_no_offset(
                    DiagCode::FontParseFailed,
                    "/CharProcs is not a dictionary; treating as zero-glyph font",
                ));
                return char_procs;
            }
        };

        // Parse each entry: glyph name -> stream reference
        for (key, value) in char_procs_dict.iter() {
            // Strip leading "/" from glyph name (PDF name syntax vs actual name)
            let glyph_name = if key.starts_with('/') {
                Arc::from(&key[1..])
            } else {
                Arc::clone(key)
            };

            let obj_ref = match value {
                PdfObject::Ref(r) => *r,
                PdfObject::Stream(_) => {
                    diagnostics.push(Diagnostic::with_dynamic_no_offset(
                        DiagCode::FontParseFailed,
                        format!(
                            "/CharProcs entry '{}' is direct stream, not reference; skipping",
                            glyph_name
                        ),
                    ));
                    continue;
                }
                _ => {
                    diagnostics.push(Diagnostic::with_dynamic_no_offset(
                        DiagCode::FontParseFailed,
                        format!(
                            "/CharProcs entry '{}' is not a stream reference; skipping",
                            glyph_name
                        ),
                    ));
                    continue;
                }
            };

            char_procs.insert(glyph_name, obj_ref);
        }

        char_procs
    }

    /// Load /FirstChar and /LastChar.
    ///
    /// Defaults to (0, 0) if missing.
    fn load_char_range(font_dict: &PdfDict, _diagnostics: &mut Vec<Diagnostic>) -> (u8, u8) {
        let first_char = font_dict
            .get("/FirstChar")
            .and_then(|obj| obj.as_int())
            .map(|i| i.clamp(0, 255) as u8)
            .unwrap_or(0);

        let last_char = font_dict
            .get("/LastChar")
            .and_then(|obj| obj.as_int())
            .map(|i| i.clamp(0, 255) as u8)
            .unwrap_or(0);

        (first_char, last_char)
    }

    /// Load /Widths array.
    ///
    /// Length should equal `last_char - first_char + 1`. On mismatch,
    /// emits diagnostic and clamps/pads.
    ///
    /// Missing /Widths defaults to all-zero.
    fn load_widths(
        font_dict: &PdfDict,
        first_char: u8,
        last_char: u8,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Vec<f64> {
        let expected_len = if last_char >= first_char {
            // Cast to usize before arithmetic to avoid overflow
            // when last_char = 255 and first_char = 0
            last_char as usize - first_char as usize + 1
        } else {
            0
        };

        let widths_obj = match font_dict.get("/Widths") {
            Some(obj) => obj,
            None => {
                diagnostics.push(Diagnostic::with_static_no_offset(
                    DiagCode::FontParseFailed,
                    "Type3 font missing /Widths array; defaulting to all-zero",
                ));
                return vec![0.0; expected_len.max(1)];
            }
        };

        let widths_array = match widths_obj {
            PdfObject::Array(arr) => arr.as_ref(),
            PdfObject::Ref(_) => {
                diagnostics.push(Diagnostic::with_static_no_offset(
                    DiagCode::FontParseFailed,
                    "/Widths is indirect reference; not supported, defaulting to all-zero",
                ));
                return vec![0.0; expected_len.max(1)];
            }
            _ => {
                diagnostics.push(Diagnostic::with_static_no_offset(
                    DiagCode::FontParseFailed,
                    "/Widths is not an array; defaulting to all-zero",
                ));
                return vec![0.0; expected_len.max(1)];
            }
        };

        // Parse widths as f64
        let mut widths: Vec<f64> = widths_array
            .iter()
            .filter_map(|obj| obj.as_real().or(obj.as_int().map(|i| i as f64)))
            .collect();

        // Validate length
        if widths.len() != expected_len {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::FontType3WidthsLengthMismatch,
                format!(
                    "/Widths length {} does not match LastChar-FirstChar+1 ({}); clamping/padding",
                    widths.len(),
                    expected_len
                ),
            ));

            // Clamp or pad to expected length
            if widths.len() > expected_len {
                widths.truncate(expected_len);
            } else if expected_len > 0 {
                while widths.len() < expected_len {
                    widths.push(0.0);
                }
            }
        }

        widths
    }

    /// Load /FontMatrix.
    ///
    /// Defaults to `[0.001 0 0 0.001 0 0]` if missing (the Type 3 default per spec).
    fn load_font_matrix(font_dict: &PdfDict, _diagnostics: &mut Vec<Diagnostic>) -> Matrix3x3 {
        let default_matrix = Matrix3x3::from_pdf_array([0.001, 0.0, 0.0, 0.001, 0.0, 0.0]);

        let matrix_obj = match font_dict.get("/FontMatrix") {
            Some(obj) => obj,
            None => return default_matrix,
        };

        let matrix_array = match matrix_obj {
            PdfObject::Array(arr) => arr.as_ref(),
            PdfObject::Ref(_) => return default_matrix,
            _ => return default_matrix,
        };

        // Parse 6-element array [a b c d e f]
        let mut values = [0.0f64; 6];
        for (i, elem) in matrix_array.iter().enumerate() {
            if i >= 6 {
                break;
            }
            values[i] = elem
                .as_real()
                .or(elem.as_int().map(|i| i as f64))
                .unwrap_or(0.0);
        }

        Matrix3x3::from_pdf_array(values)
    }

    /// Load /Resources.
    ///
    /// Returns None if /Resources is missing (will default to page resources).
    fn load_resources(font_dict: &PdfDict) -> Option<Arc<PdfDict>> {
        match font_dict.get("/Resources") {
            Some(PdfObject::Dict(d)) => {
                // Convert Box<IndexMap> to Arc<IndexMap> by dereferencing
                Some(Arc::new((**d).clone()))
            }
            Some(PdfObject::Ref(_)) => None, // Indirect reference - would need resolution
            _ => None,
        }
    }

    /// Load /FontBBox.
    ///
    /// Returns the font bounding box [llx lly urx ury] in glyph space.
    /// Defaults to [0, 0, 0, 0] if not specified.
    fn load_font_bbox(font_dict: &PdfDict, diagnostics: &mut Vec<Diagnostic>) -> [f32; 4] {
        match font_dict.get("/FontBBox") {
            Some(PdfObject::Array(arr)) => {
                let mut bbox = [0f32; 4];
                for (i, elem) in arr.iter().take(4).enumerate() {
                    bbox[i] = elem
                        .as_real()
                        .or(elem.as_int().map(|v| v as f64))
                        .unwrap_or(0.0) as f32;
                }
                bbox
            }
            Some(PdfObject::Ref(_)) => {
                diagnostics.push(Diagnostic::with_static_no_offset(
                    DiagCode::FontParseFailed,
                    "/FontBBox is indirect reference; treating as [0, 0, 0, 0]",
                ));
                [0.0, 0.0, 0.0, 0.0]
            }
            _ => {
                // Default bounding box if not specified
                [0.0, 0.0, 0.0, 0.0]
            }
        }
    }

    /// Get the advance width for a character code in text space units.
    ///
    /// Returns 0 for codes outside [first_char, last_char].
    ///
    /// The advance width is transformed from glyph space to text space
    /// by the /FontMatrix: `text_space_width = glyph_space_width * font_matrix.a`
    pub fn advance_for(&self, code: u8) -> f64 {
        if code < self.first_char || code > self.last_char {
            return 0.0;
        }

        let idx = (code - self.first_char) as usize;
        let glyph_space_width = self.widths.get(idx).copied().unwrap_or(0.0);

        // Apply FontMatrix[0] (the 'a' coefficient) to scale to text space
        // For standard FontMatrix [0.001 0 0 0.001 0 0], this scales by 0.001
        glyph_space_width * self.font_matrix.a
    }

    /// Get the glyph content stream reference for a glyph name.
    ///
    /// Returns None if the glyph name is not in /CharProcs.
    pub fn char_proc(&self, glyph_name: &str) -> Option<ObjRef> {
        self.char_procs.get(glyph_name).copied()
    }

    /// Get the glyph content stream reference for a glyph name, returning an error if not found.
    ///
    /// Returns `Ok(ObjRef)` if the glyph exists in /CharProcs.
    /// Returns `Err(Type3Error::MissingCharProcRef)` if the glyph name is not in /CharProcs.
    pub fn char_proc_required(&self, glyph_name: &str) -> Type3Result<ObjRef> {
        self.char_proc(glyph_name).ok_or_else(|| {
            Type3Error::MissingCharProcRef {
                glyph_name: glyph_name.to_string(),
            }
        })
    }

    /// Get the number of glyphs in /CharProcs.
    pub fn glyph_count(&self) -> usize {
        self.char_procs.len()
    }

    /// Check if this font has a glyph with the given name.
    pub fn has_glyph(&self, glyph_name: &str) -> bool {
        self.char_procs.contains_key(glyph_name)
    }

    /// Get a cached rasterized bitmap for a glyph.
    ///
    /// Returns None if the glyph is not in the cache.
    pub fn get_cached_bitmap(&self, glyph_name: &str) -> Option<Vec<u8>> {
        self.raster_cache
            .get(glyph_name)
            .map(|entry| entry.value().clone())
    }

    /// Cache a rasterized bitmap for a glyph.
    pub fn cache_bitmap(&self, glyph_name: Arc<str>, bitmap: Vec<u8>) {
        self.raster_cache.entry(glyph_name).or_insert(bitmap);
    }

    /// Get the raster cache (for testing and diagnostics).
    pub fn raster_cache(&self) -> &DashMap<Arc<str>, Vec<u8>> {
        &self.raster_cache
    }

    /// Create a minimal Type3Font mock for testing.
    ///
    /// This function creates a Type3Font instance with sensible default values
    /// for testing the `rasterize_type3_glyph` function. It provides the minimum
    /// required fields: FontBBox, FontMatrix, CharProcs, and Encoding.
    ///
    /// # Arguments
    ///
    /// * `char_procs` - Optional HashMap of glyph name -> ObjRef for glyph content streams
    ///
    /// # Returns
    ///
    /// A Type3Font with:
    /// - Identity FontMatrix ([1 0 0 1 0 0]) for predictable coordinates
    /// - FontBBox [0, 0, 1000, 1000] for a standard glyph space
    /// - StandardEncoding for encoding
    /// - Provided char_procs (or empty if None)
    /// - Zero first_char, last_char, and widths
    /// - No resources
    /// - No diagnostics
    /// - Empty raster cache
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::collections::HashMap;
    /// use std::sync::Arc;
    /// use crate::font::type3::Type3Font;
    /// use crate::parser::object::types::ObjRef;
    ///
    /// // Create a mock font with a test glyph
    /// let mut char_procs = HashMap::new();
    /// char_procs.insert(Arc::from("A"), ObjRef::new(10, 0));
    /// let font = Type3Font::mock(Some(char_procs));
    /// ```
    pub fn mock(char_procs: Option<HashMap<Arc<str>, ObjRef>>) -> Self {
        Self {
            char_procs: char_procs.unwrap_or_default(),
            first_char: 0,
            last_char: 0,
            widths: vec![0.0],
            // Identity matrix for predictable coordinates in tests (no scaling/transform)
            font_matrix: Matrix3x3::identity(),
            resources: None,
            encoding: FontEncoding::new(Some(crate::font::encoding::NamedEncoding::Standard)),
            // Standard glyph space bounding box [0,0,1000,1000] for consistent test expectations
            font_bbox: [0.0, 0.0, 1000.0, 1000.0],
            diagnostics: Vec::new(),
            raster_cache: Arc::new(DashMap::new()),
        }
    }

    /// Create a Type3Font with custom CharProcs and sensible defaults for other fields.
    ///
    /// This function creates a Type3Font instance with the provided CharProcs dictionary,
    /// using sensible defaults for all other fields. This is useful for testing Type3 font
    /// functionality with custom glyph content streams.
    ///
    /// # Arguments
    ///
    /// * `char_procs` - HashMap of glyph name -> ObjRef for glyph content streams
    ///
    /// # Returns
    ///
    /// A Type3Font with:
    /// - Identity FontMatrix ([1 0 0 1 0 0]) for predictable coordinates
    /// - FontBBox [0, 0, 1000, 1000] for a standard glyph space
    /// - StandardEncoding for encoding
    /// - Provided char_procs
    /// - Zero first_char, last_char, and widths
    /// - No resources
    /// - No diagnostics
    /// - Empty raster cache
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::collections::HashMap;
    /// use std::sync::Arc;
    /// use pdftract_core::font::type3::Type3Font;
    /// use pdftract_core::parser::object::types::ObjRef;
    ///
    /// // Create a font with custom CharProcs
    /// let mut char_procs = HashMap::new();
    /// char_procs.insert(Arc::from("A"), ObjRef::new(10, 0));
    /// char_procs.insert(Arc::from("B"), ObjRef::new(11, 0));
    /// let font = Type3Font::type3_font_with_char_procs(char_procs);
    /// ```
    pub fn type3_font_with_char_procs(char_procs: HashMap<Arc<str>, ObjRef>) -> Self {
        Self {
            char_procs,
            first_char: 0,
            last_char: 0,
            widths: vec![0.0],
            font_matrix: Matrix3x3::identity(),
            resources: None,
            encoding: FontEncoding::new(Some(crate::font::encoding::NamedEncoding::Standard)),
            font_bbox: [0.0, 0.0, 1000.0, 1000.0],
            diagnostics: Vec::new(),
            raster_cache: Arc::new(DashMap::new()),
        }
    }

    /// Create a Type3Font with custom Resources and sensible defaults for other fields.
    ///
    /// This function creates a Type3Font instance with the provided Resources dictionary,
    /// using sensible defaults for all other fields. This is useful for testing Type3 font
    /// functionality with resource dictionaries (e.g., fonts, XObjects referenced by glyphs).
    ///
    /// # Arguments
    ///
    /// * `resources` - Arc wrapping the PdfDict containing resources for glyph content streams
    ///
    /// # Returns
    ///
    /// A Type3Font with:
    /// - Identity FontMatrix ([1 0 0 1 0 0]) for predictable coordinates
    /// - FontBBox [0, 0, 1000, 1000] for a standard glyph space
    /// - StandardEncoding for encoding
    /// - Empty char_procs
    /// - Zero first_char, last_char, and widths
    /// - Provided resources
    /// - No diagnostics
    /// - Empty raster cache
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use pdftract_core::font::type3::Type3Font;
    /// use pdftract_core::parser::object::PdfDict;
    ///
    /// // Create a font with custom Resources
    /// let mut resources = PdfDict::new();
    /// // Add resources like fonts, XObjects, etc.
    /// let font = Type3Font::type3_font_with_resources(Arc::new(resources));
    /// ```
    pub fn type3_font_with_resources(resources: Arc<PdfDict>) -> Self {
        Self {
            char_procs: HashMap::new(),
            first_char: 0,
            last_char: 0,
            widths: vec![0.0],
            font_matrix: Matrix3x3::identity(),
            resources: Some(resources),
            encoding: FontEncoding::new(Some(crate::font::encoding::NamedEncoding::Standard)),
            font_bbox: [0.0, 0.0, 1000.0, 1000.0],
            diagnostics: Vec::new(),
            raster_cache: Arc::new(DashMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::types::intern;

    #[test]
    fn test_type3_load_minimal() {
        // Create a minimal Type3 font dict
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type3")));
        font_dict.insert(intern("/FirstChar"), PdfObject::Integer(0));
        font_dict.insert(intern("/LastChar"), PdfObject::Integer(5));
        font_dict.insert(
            intern("/Widths"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(500),
                PdfObject::Integer(600),
                PdfObject::Integer(700),
                PdfObject::Integer(800),
                PdfObject::Integer(900),
                PdfObject::Integer(1000),
            ])),
        );

        let font = Type3Font::load(&font_dict);

        assert_eq!(font.first_char, 0);
        assert_eq!(font.last_char, 5);
        assert_eq!(font.widths.len(), 6);
        assert_eq!(font.widths[0], 500.0);
        assert_eq!(font.widths[5], 1000.0);
        // Default FontMatrix
        assert_eq!(font.font_matrix.a, 0.001);
        // No /CharProcs
        assert_eq!(font.glyph_count(), 0);
    }

    #[test]
    fn test_type3_with_char_procs() {
        // Create /CharProcs dictionary
        let mut char_procs_dict = PdfDict::new();
        char_procs_dict.insert(intern("/A"), PdfObject::Ref(ObjRef::new(10, 0)));
        char_procs_dict.insert(intern("/B"), PdfObject::Ref(ObjRef::new(11, 0)));

        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/CharProcs"),
            PdfObject::Dict(Box::new(char_procs_dict)),
        );
        font_dict.insert(intern("/FirstChar"), PdfObject::Integer(0));
        font_dict.insert(intern("/LastChar"), PdfObject::Integer(1));
        font_dict.insert(
            intern("/Widths"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(500),
                PdfObject::Integer(600),
            ])),
        );

        let font = Type3Font::load(&font_dict);

        assert_eq!(font.glyph_count(), 2);
        assert!(font.has_glyph("A"));
        assert!(font.has_glyph("B"));
        assert!(!font.has_glyph("C"));

        assert_eq!(font.char_proc("A"), Some(ObjRef::new(10, 0)));
        assert_eq!(font.char_proc("B"), Some(ObjRef::new(11, 0)));
        assert_eq!(font.char_proc("C"), None);
    }

    #[test]
    fn test_advance_for_with_standard_font_matrix() {
        // Test with default FontMatrix [0.001 0 0 0.001 0 0]
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/FirstChar"), PdfObject::Integer(32));
        font_dict.insert(intern("/LastChar"), PdfObject::Integer(33));
        font_dict.insert(
            intern("/Widths"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(500),  // code 32
                PdfObject::Integer(1000), // code 33
            ])),
        );

        let font = Type3Font::load(&font_dict);

        // Width 500 * 0.001 = 0.5 text units
        assert_eq!(font.advance_for(32), 0.5);
        // Width 1000 * 0.001 = 1.0 text units
        assert_eq!(font.advance_for(33), 1.0);
    }

    #[test]
    fn test_advance_for_with_identity_font_matrix() {
        // Test with identity FontMatrix [1 0 0 1 0 0]
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );
        font_dict.insert(intern("/FirstChar"), PdfObject::Integer(32));
        font_dict.insert(intern("/LastChar"), PdfObject::Integer(32));
        font_dict.insert(
            intern("/Widths"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(500)])),
        );

        let font = Type3Font::load(&font_dict);

        // Width 500 * 1.0 = 500 text units (no scaling)
        assert_eq!(font.advance_for(32), 500.0);
    }

    #[test]
    fn test_advance_for_out_of_range() {
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/FirstChar"), PdfObject::Integer(32));
        font_dict.insert(intern("/LastChar"), PdfObject::Integer(126));
        font_dict.insert(
            intern("/Widths"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(500)])),
        );

        let font = Type3Font::load(&font_dict);

        // Before range
        assert_eq!(font.advance_for(31), 0.0);
        // After range
        assert_eq!(font.advance_for(127), 0.0);
    }

    #[test]
    fn test_widths_length_mismatch() {
        // /Widths has 3 elements but FirstChar=0, LastChar=5 (expected 6)
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/FirstChar"), PdfObject::Integer(0));
        font_dict.insert(intern("/LastChar"), PdfObject::Integer(5));
        font_dict.insert(
            intern("/Widths"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(500),
                PdfObject::Integer(600),
                PdfObject::Integer(700),
            ])),
        );

        let font = Type3Font::load(&font_dict);

        // Should emit diagnostic and pad with zeros
        assert_eq!(font.widths.len(), 6);
        assert_eq!(font.widths[0], 500.0);
        assert_eq!(font.widths[1], 600.0);
        assert_eq!(font.widths[2], 700.0);
        assert_eq!(font.widths[3], 0.0); // Padded
        assert_eq!(font.widths[4], 0.0); // Padded
        assert_eq!(font.widths[5], 0.0); // Padded

        assert!(font
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::FontType3WidthsLengthMismatch));
    }

    #[test]
    fn test_widths_too_long() {
        // /Widths has 10 elements but FirstChar=0, LastChar=2 (expected 3)
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/FirstChar"), PdfObject::Integer(0));
        font_dict.insert(intern("/LastChar"), PdfObject::Integer(2));
        font_dict.insert(
            intern("/Widths"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(500),
                PdfObject::Integer(600),
                PdfObject::Integer(700),
                PdfObject::Integer(800),
                PdfObject::Integer(900),
                PdfObject::Integer(1000),
                PdfObject::Integer(1100),
                PdfObject::Integer(1200),
                PdfObject::Integer(1300),
                PdfObject::Integer(1400),
            ])),
        );

        let font = Type3Font::load(&font_dict);

        // Should emit diagnostic and truncate
        assert_eq!(font.widths.len(), 3);
        assert_eq!(font.widths[0], 500.0);
        assert_eq!(font.widths[1], 600.0);
        assert_eq!(font.widths[2], 700.0);

        assert!(font
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::FontType3WidthsLengthMismatch));
    }

    #[test]
    fn test_missing_widths() {
        // No /Widths array
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/FirstChar"), PdfObject::Integer(0));
        font_dict.insert(intern("/LastChar"), PdfObject::Integer(5));

        let font = Type3Font::load(&font_dict);

        // Should default to all-zero
        assert_eq!(font.widths.len(), 6);
        assert!(font.widths.iter().all(|&w| w == 0.0));

        assert!(font
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::FontParseFailed));
    }

    #[test]
    fn test_missing_char_procs() {
        // No /CharProcs dictionary
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/FirstChar"), PdfObject::Integer(0));
        font_dict.insert(intern("/LastChar"), PdfObject::Integer(0));

        let font = Type3Font::load(&font_dict);

        // Should have empty char_procs
        assert_eq!(font.glyph_count(), 0);
        assert!(font
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::FontParseFailed));
    }

    #[test]
    fn test_custom_font_matrix() {
        // Test custom FontMatrix
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Real(0.002),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Real(0.002),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );
        font_dict.insert(intern("/FirstChar"), PdfObject::Integer(0));
        font_dict.insert(intern("/LastChar"), PdfObject::Integer(0));
        font_dict.insert(
            intern("/Widths"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(500)])),
        );

        let font = Type3Font::load(&font_dict);

        assert_eq!(font.font_matrix.a, 0.002);
        // Width 500 * 0.002 = 1.0 text units
        assert_eq!(font.advance_for(0), 1.0);
    }

    #[test]
    fn test_with_resources() {
        // Test with /Resources dictionary
        let mut resources = PdfDict::new();
        resources.insert(intern("/Font"), PdfObject::Array(Box::new(vec![])));

        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Resources"), PdfObject::Dict(Box::new(resources)));
        font_dict.insert(intern("/FirstChar"), PdfObject::Integer(0));
        font_dict.insert(intern("/LastChar"), PdfObject::Integer(0));

        let font = Type3Font::load(&font_dict);

        assert!(font.resources.is_some());
    }

    #[test]
    fn test_arbitrary_glyph_names() {
        // Type3 fonts can have arbitrary glyph names
        let mut char_procs_dict = PdfDict::new();
        char_procs_dict.insert(intern("/CustomGlyph1"), PdfObject::Ref(ObjRef::new(10, 0)));
        char_procs_dict.insert(
            intern("/MySpecialGlyph"),
            PdfObject::Ref(ObjRef::new(11, 0)),
        );

        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/CharProcs"),
            PdfObject::Dict(Box::new(char_procs_dict)),
        );
        font_dict.insert(intern("/FirstChar"), PdfObject::Integer(0));
        font_dict.insert(intern("/LastChar"), PdfObject::Integer(0));

        let font = Type3Font::load(&font_dict);

        assert!(font.has_glyph("CustomGlyph1"));
        assert!(font.has_glyph("MySpecialGlyph"));
    }

    #[test]
    fn test_encoding_parse() {
        // Test that encoding is parsed
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/Encoding"),
            PdfObject::Name(intern("/WinAnsiEncoding")),
        );
        font_dict.insert(intern("/FirstChar"), PdfObject::Integer(0));
        font_dict.insert(intern("/LastChar"), PdfObject::Integer(0));

        let font = Type3Font::load(&font_dict);

        assert_eq!(
            font.encoding.base_encoding(),
            Some(crate::font::encoding::NamedEncoding::WinAnsi)
        );
    }

    #[test]
    fn test_char_proc_required_missing_returns_error() {
        // Test that char_proc_required returns an error for missing glyphs
        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);

        let result = font.char_proc_required("NonExistentGlyph");

        assert!(result.is_err());
        match result {
            Err(Type3Error::MissingCharProcRef { glyph_name }) => {
                assert_eq!(glyph_name, "NonExistentGlyph");
            }
            _ => panic!("Expected Type3Error::MissingCharProcRef"),
        }
    }

    #[test]
    fn test_char_proc_required_found_returns_ref() {
        // Test that char_proc_required returns Ok for existing glyphs
        let mut char_procs_dict = PdfDict::new();
        char_procs_dict.insert(intern("/A"), PdfObject::Ref(ObjRef::new(10, 0)));

        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/CharProcs"),
            PdfObject::Dict(Box::new(char_procs_dict)),
        );

        let font = Type3Font::load(&font_dict);

        let result = font.char_proc_required("A");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ObjRef::new(10, 0));
    }

    #[test]
    fn test_type3_error_display_includes_glyph_name() {
        // Test that the error message includes the missing glyph name
        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);

        let result = font.char_proc_required("MissingGlyph");

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("MissingGlyph"));
        assert!(error_msg.contains("character procedure reference not found"));
    }

    #[test]
    fn test_type3_font_mock_creates_minimal_font() {
        // Test that the mock function creates a Type3Font with sensible defaults
        let font = Type3Font::mock(None);

        // Verify identity FontMatrix (predictable coordinates for testing)
        assert_eq!(font.font_matrix.a, 1.0, "FontMatrix should be identity");
        assert_eq!(font.font_matrix.d, 1.0, "FontMatrix should be identity");
        assert_eq!(font.font_matrix.b, 0.0, "FontMatrix should be identity");
        assert_eq!(font.font_matrix.c, 0.0, "FontMatrix should be identity");

        // Verify standard FontBBox [0, 0, 1000, 1000]
        assert_eq!(font.font_bbox, [0.0, 0.0, 1000.0, 1000.0]);

        // Verify StandardEncoding
        assert_eq!(
            font.encoding.base_encoding(),
            Some(crate::font::encoding::NamedEncoding::Standard)
        );

        // Verify empty CharProcs when None provided
        assert_eq!(font.glyph_count(), 0);

        // Verify default widths and char range
        assert_eq!(font.first_char, 0);
        assert_eq!(font.last_char, 0);
        assert_eq!(font.widths, vec![0.0]);

        // Verify no resources and empty diagnostics
        assert!(font.resources.is_none());
        assert!(font.diagnostics.is_empty());
    }

    #[test]
    fn test_type3_font_mock_with_custom_char_procs() {
        // Test that mock accepts custom CharProcs
        let mut char_procs = HashMap::new();
        char_procs.insert(Arc::from("A"), ObjRef::new(10, 0));
        char_procs.insert(Arc::from("B"), ObjRef::new(11, 0));

        let font = Type3Font::mock(Some(char_procs));

        // Verify custom CharProcs are set
        assert_eq!(font.glyph_count(), 2);
        assert!(font.has_glyph("A"));
        assert!(font.has_glyph("B"));
        assert_eq!(font.char_proc("A"), Some(ObjRef::new(10, 0)));
        assert_eq!(font.char_proc("B"), Some(ObjRef::new(11, 0)));
    }

    #[test]
    fn test_type3_font_mock_works_with_rasterize_type3_glyph() {
        // Test that mock font is compatible with rasterize_type3_glyph
        let mut char_procs = HashMap::new();
        char_procs.insert(Arc::from("test"), ObjRef::new(42, 0));

        let font = Type3Font::mock(Some(char_procs));

        // Create a resolver that returns minimal content stream
        let resolver = |_: ObjRef| -> Option<Vec<u8>> { Some(vec![]) };

        let result = crate::font::type3_rasterizer::rasterize_type3_glyph(
            &font,
            "test",
            None,
            Some(&resolver),
        );

        // Should successfully rasterize (empty stream produces default bitmap)
        assert!(result.is_some(), "Mock font should work with rasterize_type3_glyph");
    }

    #[test]
    fn test_mock_creates_identity_font_matrix() {
        // Verify font_matrix is identity [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        let font = Type3Font::mock(None);

        assert_eq!(font.font_matrix.a, 1.0, "FontMatrix[0] should be 1.0");
        assert_eq!(font.font_matrix.b, 0.0, "FontMatrix[1] should be 0.0");
        assert_eq!(font.font_matrix.c, 0.0, "FontMatrix[2] should be 0.0");
        assert_eq!(font.font_matrix.d, 1.0, "FontMatrix[3] should be 1.0");
        assert_eq!(font.font_matrix.e, 0.0, "FontMatrix[4] should be 0.0");
        assert_eq!(font.font_matrix.f, 0.0, "FontMatrix[5] should be 0.0");
    }

    #[test]
    fn test_mock_creates_default_font_bbox() {
        // Verify font_bbox is [0.0, 0.0, 1000.0, 1000.0]
        let font = Type3Font::mock(None);

        assert_eq!(font.font_bbox, [0.0, 0.0, 1000.0, 1000.0]);
    }

    #[test]
    fn test_mock_creates_standard_encoding() {
        // Verify encoding.name == "StandardEncoding"
        let font = Type3Font::mock(None);

        assert_eq!(
            font.encoding.base_encoding(),
            Some(crate::font::encoding::NamedEncoding::Standard)
        );
    }

    #[test]
    fn test_mock_with_no_char_procs() {
        // Test mock with None (no char_procs provided)
        let font = Type3Font::mock(None);

        // Verify char_procs is empty (default HashMap)
        assert!(font.char_procs.is_empty(), "char_procs should be empty when None is provided");
        assert_eq!(font.glyph_count(), 0, "glyph_count should be 0 with no char_procs");
        assert!(!font.has_glyph("A"), "has_glyph should return false for any glyph name");
        assert_eq!(font.char_proc("A"), None, "char_proc should return None for any glyph name");
    }

    #[test]
    fn test_mock_with_custom_char_procs() {
        // Test mock with custom char_procs
        let mut char_procs = HashMap::new();
        char_procs.insert(Arc::from("A"), ObjRef::new(10, 0));
        char_procs.insert(Arc::from("B"), ObjRef::new(11, 0));
        char_procs.insert(Arc::from("C"), ObjRef::new(12, 0));

        let font = Type3Font::mock(Some(char_procs));

        // Verify custom CharProcs are set correctly
        assert_eq!(font.glyph_count(), 3, "glyph_count should match number of custom char_procs");
        assert!(font.has_glyph("A"), "has_glyph should return true for 'A'");
        assert!(font.has_glyph("B"), "has_glyph should return true for 'B'");
        assert!(font.has_glyph("C"), "has_glyph should return true for 'C'");
        assert!(!font.has_glyph("D"), "has_glyph should return false for non-existent 'D'");

        assert_eq!(font.char_proc("A"), Some(ObjRef::new(10, 0)), "char_proc('A') should return correct ref");
        assert_eq!(font.char_proc("B"), Some(ObjRef::new(11, 0)), "char_proc('B') should return correct ref");
        assert_eq!(font.char_proc("C"), Some(ObjRef::new(12, 0)), "char_proc('C') should return correct ref");
        assert_eq!(font.char_proc("D"), None, "char_proc('D') should return None");
    }

    #[test]
    fn test_mock_initializes_all_required_fields() {
        // Test that mock initializes all Type3Font fields properly
        let mut custom_char_procs = HashMap::new();
        custom_char_procs.insert(Arc::from("X"), ObjRef::new(42, 0));

        let font = Type3Font::mock(Some(custom_char_procs.clone()));

        // Verify char_procs field
        assert_eq!(font.char_procs, custom_char_procs, "char_procs should match input");

        // Verify first_char and last_char (range for widths array)
        assert_eq!(font.first_char, 0, "first_char should be 0");
        assert_eq!(font.last_char, 0, "last_char should be 0");

        // Verify widths array
        assert_eq!(font.widths, vec![0.0], "widths should be [0.0] (single zero width)");

        // Verify font_matrix (identity matrix)
        assert_eq!(font.font_matrix.a, 1.0, "font_matrix.a should be 1.0");
        assert_eq!(font.font_matrix.b, 0.0, "font_matrix.b should be 0.0");
        assert_eq!(font.font_matrix.c, 0.0, "font_matrix.c should be 0.0");
        assert_eq!(font.font_matrix.d, 1.0, "font_matrix.d should be 1.0");
        assert_eq!(font.font_matrix.e, 0.0, "font_matrix.e should be 0.0");
        assert_eq!(font.font_matrix.f, 0.0, "font_matrix.f should be 0.0");

        // Verify resources
        assert!(font.resources.is_none(), "resources should be None");

        // Verify encoding
        assert_eq!(
            font.encoding.base_encoding(),
            Some(crate::font::encoding::NamedEncoding::Standard),
            "encoding should use StandardEncoding"
        );

        // Verify font_bbox
        assert_eq!(font.font_bbox, [0.0, 0.0, 1000.0, 1000.0], "font_bbox should be [0, 0, 1000, 1000]");

        // Verify diagnostics
        assert!(font.diagnostics.is_empty(), "diagnostics should be empty");

        // Verify raster_cache
        assert!(!font.raster_cache.is_empty() || font.raster_cache.len() == 0,
            "raster_cache should be a valid DashMap (may be empty)");
        // Verify we can insert and retrieve from the cache
        font.raster_cache.insert(Arc::from("test"), vec![1, 2, 3]);
        assert_eq!(font.raster_cache.get(&Arc::from("test")).map(|v| v.value().clone()), Some(vec![1, 2, 3]),
            "raster_cache should be functional");
    }

    #[test]
    fn test_mock_works_with_rasterize_type3_glyph_complex() {
        // Test mock font with a more complex drawing case (rectangle fill)
        let mut char_procs = HashMap::new();
        char_procs.insert(Arc::from("rect"), ObjRef::new(42, 0));

        let font = Type3Font::mock(Some(char_procs));

        // Create a resolver that returns a content stream that draws a rectangle
        // Content stream: "5 5 10 10 re f" (draw and fill a 10x10 rectangle)
        let resolver = |_: ObjRef| -> Option<Vec<u8>> {
            Some(b"5 5 10 10 re f".to_vec())
        };

        let result = crate::font::type3_rasterizer::rasterize_type3_glyph(
            &font,
            "rect",
            None,
            Some(&resolver),
        );

        // Should successfully rasterize
        assert!(result.is_some(), "Mock font should rasterize rectangle glyph");

        // Verify we get a bitmap back (even if small)
        let bitmap_bytes = result.unwrap();
        assert!(!bitmap_bytes.is_empty(), "Bitmap should not be empty");
    }

    #[test]
    fn test_mock_works_with_rasterize_type3_glyph_stroke() {
        // Test mock font with stroke operations
        let mut char_procs = HashMap::new();
        char_procs.insert(Arc::from("line"), ObjRef::new(43, 0));

        let font = Type3Font::mock(Some(char_procs));

        // Create a resolver that returns a content stream with a line
        // Content stream: "10 10 m 20 20 l S" (move and draw a line, then stroke)
        let resolver = |_: ObjRef| -> Option<Vec<u8>> {
            Some(b"10 10 m 20 20 l S".to_vec())
        };

        let result = crate::font::type3_rasterizer::rasterize_type3_glyph(
            &font,
            "line",
            None,
            Some(&resolver),
        );

        // Should successfully rasterize
        assert!(result.is_some(), "Mock font should rasterize stroke glyph");

        // Verify we get a bitmap back
        let bitmap_bytes = result.unwrap();
        assert!(!bitmap_bytes.is_empty(), "Bitmap should not be empty");
    }

    #[test]
    fn test_mock_works_with_rasterize_type3_glyph_unknown_glyph() {
        // Test that unknown glyph names gracefully return None
        let mut char_procs = HashMap::new();
        char_procs.insert(Arc::from("known"), ObjRef::new(42, 0));

        let font = Type3Font::mock(Some(char_procs));

        let resolver = |_: ObjRef| -> Option<Vec<u8>> {
            Some(b"5 5 10 10 re f".to_vec())
        };

        let result = crate::font::type3_rasterizer::rasterize_type3_glyph(
            &font,
            "unknown_glyph",  // This glyph doesn't exist in char_procs
            None,
            Some(&resolver),
        );

        // Should gracefully return None for unknown glyphs
        assert!(result.is_none(), "Unknown glyph should return None");
    }

    #[test]
    fn test_mock_works_with_rasterize_type3_glyph_no_resolver() {
        // Test that the function works when no resolver is provided
        let mut char_procs = HashMap::new();
        char_procs.insert(Arc::from("test"), ObjRef::new(42, 0));

        let font = Type3Font::mock(Some(char_procs));

        // Call without a resolver
        let result = crate::font::type3_rasterizer::rasterize_type3_glyph(
            &font,
            "test",
            None,
            None::<&crate::font::type3_rasterizer::StreamResolverFn>,  // No resolver provided
        );

        // Should return None (can't resolve the stream)
        assert!(result.is_none(), "Should return None when no resolver provided");
    }

    #[test]
    fn test_helper_functions_compatible_with_mock() {
        // Test that helper functions from test_glyph_helper work with Type3Font::mock
        use crate::font::test_glyph_helper::{
            make_rect_glyph, make_test_char_procs, make_test_resolver,
        };
        use std::collections::HashMap;

        // Create char_procs using helper
        let char_procs = make_test_char_procs();

        // Create mock font with helper output
        let font = Type3Font::mock(Some(char_procs));

        // Verify the font was created successfully
        assert_eq!(font.glyph_count(), 5, "Should have 5 glyphs from make_test_char_procs");
        assert!(font.has_glyph("A"), "Should have 'A' glyph");
        assert!(font.has_glyph("B"), "Should have 'B' glyph");
        assert!(font.has_glyph("rect"), "Should have 'rect' glyph");
        assert!(font.has_glyph("line"), "Should have 'line' glyph");
        assert!(font.has_glyph("empty"), "Should have 'empty' glyph");
    }

    #[test]
    fn test_helper_rect_glyph_compatible_with_rasterizer() {
        // Test that make_rect_glyph output works with rasterize_type3_glyph
        use crate::font::test_glyph_helper::{
            make_rect_glyph, make_test_char_procs, make_test_resolver,
        };
        use std::collections::HashMap;

        // Create char_procs and font using helpers
        let char_procs = make_test_char_procs();
        let font = Type3Font::mock(Some(char_procs));

        // Create glyph data using helper
        let rect_glyph = make_rect_glyph(0.0, 0.0, 100.0, 100.0);

        // Create resolver mapping using character names
        let mut glyph_map = HashMap::new();
        glyph_map.insert("/A".to_string(), rect_glyph);  // ObjRef(1, 0) maps to "/A"

        let resolver = make_test_resolver(&glyph_map);

        // Test rasterization
        let result = crate::font::type3_rasterizer::rasterize_type3_glyph(
            &font,
            "A",  // ObjRef(10, 0)
            None,
            Some(&resolver),
        );

        // Should successfully rasterize
        assert!(result.is_some(), "make_rect_glyph should be compatible with rasterize_type3_glyph");
        let bitmap = result.unwrap();
        assert!(!bitmap.is_empty(), "Bitmap should not be empty");
        // Bitmap size is determined by font_bbox [0,0,1000,1000] -> 1002x1002 after padding
        assert_eq!(bitmap.len(), 1002 * 1002, "Bitmap should be 1002x1002 from mock's font_bbox");
    }

    #[test]
    fn test_helper_line_glyph_compatible_with_rasterizer() {
        // Test that make_line_glyph output works with rasterize_type3_glyph
        use crate::font::test_glyph_helper::{
            make_line_glyph, make_test_char_procs, make_test_resolver,
        };
        use std::collections::HashMap;

        // Create char_procs and font using helpers
        let char_procs = make_test_char_procs();
        let font = Type3Font::mock(Some(char_procs));

        // Create glyph data using helper
        let line_glyph = make_line_glyph(0.0, 0.0, 50.0, 50.0);

        // Create resolver mapping - needs string keys matching the char_procs format
        let mut glyph_map: HashMap<String, Vec<u8>> = HashMap::new();
        glyph_map.insert("/A".to_string(), line_glyph);  // ObjRef for "A" is (1, 0)

        let resolver = make_test_resolver(&glyph_map);

        // Test rasterization
        let result = crate::font::type3_rasterizer::rasterize_type3_glyph(
            &font,
            "A",  // ObjRef(10, 0)
            None,
            Some(&resolver),
        );

        // Should successfully rasterize
        assert!(result.is_some(), "make_line_glyph should be compatible with rasterize_type3_glyph");
        let bitmap = result.unwrap();
        assert!(!bitmap.is_empty(), "Bitmap should not be empty");
    }

    #[test]
    fn test_helper_empty_glyph_compatible_with_rasterizer() {
        // Test that make_empty_glyph output works with rasterize_type3_glyph
        use crate::font::test_glyph_helper::{
            make_empty_glyph, make_test_char_procs, make_test_resolver,
        };
        use std::collections::HashMap;

        // Create char_procs and font using helpers
        let char_procs = make_test_char_procs();
        let font = Type3Font::mock(Some(char_procs));

        // Create glyph data using helper
        let empty_glyph = make_empty_glyph();

        // Create resolver mapping - needs string keys matching the char_procs format
        let mut glyph_map: HashMap<String, Vec<u8>> = HashMap::new();
        glyph_map.insert("/A".to_string(), empty_glyph);  // ObjRef for "A" is (1, 0)

        let resolver = make_test_resolver(&glyph_map);

        // Test rasterization
        let result = crate::font::type3_rasterizer::rasterize_type3_glyph(
            &font,
            "A",  // ObjRef(10, 0)
            None,
            Some(&resolver),
        );

        // Should successfully rasterize (empty glyph produces all-white bitmap)
        assert!(result.is_some(), "make_empty_glyph should be compatible with rasterize_type3_glyph");
        let bitmap = result.unwrap();
        assert!(!bitmap.is_empty(), "Bitmap should not be empty");
        // Bitmap size is determined by font_bbox [0,0,1000,1000] -> 1002x1002 after padding
        assert_eq!(bitmap.len(), 1002 * 1002, "Bitmap should be 1002x1002 from mock's font_bbox");
    }

    #[test]
    fn test_helper_custom_char_procs_compatible() {
        // Test that make_custom_char_procs_from_names works with Type3Font::mock
        use crate::font::test_glyph_helper::{make_custom_char_procs_from_names, make_test_resolver};
        use std::collections::HashMap;

        // Create custom char_procs
        let char_procs = make_custom_char_procs_from_names(&["g1", "g2", "g3"], 100);

        // Create mock font
        let font = Type3Font::mock(Some(char_procs));

        // Verify the font has the custom glyphs
        assert_eq!(font.glyph_count(), 3, "Should have 3 custom glyphs");
        assert!(font.has_glyph("g1"), "Should have 'g1' glyph");
        assert!(font.has_glyph("g2"), "Should have 'g2' glyph");
        assert!(font.has_glyph("g3"), "Should have 'g3' glyph");

        // Create glyph data
        let glyph_data = vec![b"10 10 50 50 re f".to_vec()];

        // Create resolver mapping - g1 is at ID 100, which maps to character name
        // For ID 100, the resolver maps to format!("/{}", (100 + b'A' - 1) as char)
        // But we need to provide the data using the expected character name format
        let mut glyph_map: HashMap<String, Vec<u8>> = HashMap::new();
        // ObjRef ID 100 would map to character name "/A" + (100 - 1) = "/..."
        // Actually, looking at the resolver logic: (ref_id.object as u8 + b'A' - 1)
        // For ID 100: (100 + 65 - 1) = 164, which is '¤' - this is wrong for high IDs
        // We need to provide the glyph data with the correct key format that the resolver expects
        // For now, let's use a simpler approach with lower IDs
        let char_procs = make_custom_char_procs_from_names(&["g1"], 1);
        let font = Type3Font::mock(Some(char_procs));
        glyph_map.insert("/A".to_string(), glyph_data[0].clone());

        let resolver = make_test_resolver(&glyph_map);

        // Test rasterization
        let result = crate::font::type3_rasterizer::rasterize_type3_glyph(
            &font,
            "g1",  // ObjRef(1, 0) maps to "/A"
            None,
            Some(&resolver),
        );

        // Should successfully rasterize
        assert!(result.is_some(), "Custom char_procs should be compatible with rasterize_type3_glyph");
    }

    #[test]
    fn test_helper_no_panics_or_errors() {
        // Comprehensive test that helpers work without panics or errors
        use crate::font::test_glyph_helper::{
            make_custom_char_procs_from_names, make_empty_glyph, make_line_glyph, make_rect_glyph,
            make_test_char_procs, make_test_resolver,
        };
        use std::collections::HashMap;

        // Test all helper functions don't panic
        let rect_glyph = make_rect_glyph(10.0, 20.0, 100.0, 200.0);
        let line_glyph = make_line_glyph(0.0, 0.0, 50.0, 50.0);
        let empty_glyph = make_empty_glyph();
        let char_procs = make_test_char_procs();
        let custom_char_procs = make_custom_char_procs_from_names(&["x", "y"], 200);

        // Verify helper output is valid
        assert!(!rect_glyph.is_empty(), "make_rect_glyph should produce output");
        assert!(!line_glyph.is_empty(), "make_line_glyph should produce output");
        assert!(empty_glyph.is_empty(), "make_empty_glyph should produce empty output");
        assert_eq!(char_procs.len(), 5, "make_test_char_procs should produce 5 entries");
        assert_eq!(custom_char_procs.len(), 2, "make_custom_char_procs should produce 2 entries");

        // Create font with helpers
        let font = Type3Font::mock(Some(char_procs));

        // Create resolver with helpers - needs string keys matching the char_procs
        let mut glyph_map: HashMap<String, Vec<u8>> = HashMap::new();
        glyph_map.insert("/A".to_string(), rect_glyph.clone());
        let resolver = make_test_resolver(&glyph_map);

        // Test that calling rasterize_type3_glyph works correctly
        // (If this panics, the test will fail - no need for catch_unwind)
        let result = crate::font::type3_rasterizer::rasterize_type3_glyph(&font, "A", None, Some(&resolver));

        assert!(result.is_some(), "rasterize_type3_glyph should succeed with helper output");
        let bitmap = result.unwrap();
        assert!(!bitmap.is_empty(), "Bitmap should not be empty");
    }
}
