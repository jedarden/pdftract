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
    /// Diagnostics emitted during loading.
    pub diagnostics: Vec<Diagnostic>,
    /// Rasterized glyph cache: glyph name -> 32x32 bitmap.
    ///
    /// Cached to avoid re-rasterizing the same glyph multiple times
    /// during shape recognition.
    raster_cache: Arc<DashMap<Arc<str>, [u8; 1024]>>,
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

        Self {
            char_procs,
            first_char,
            last_char,
            widths,
            font_matrix,
            resources,
            encoding,
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
    pub fn get_cached_bitmap(&self, glyph_name: &str) -> Option<[u8; 1024]> {
        self.raster_cache
            .get(glyph_name)
            .map(|entry| *entry.value())
    }

    /// Cache a rasterized bitmap for a glyph.
    pub fn cache_bitmap(&self, glyph_name: Arc<str>, bitmap: [u8; 1024]) {
        self.raster_cache.entry(glyph_name).or_insert(bitmap);
    }

    /// Get the raster cache (for testing and diagnostics).
    pub fn raster_cache(&self) -> &DashMap<Arc<str>, [u8; 1024]> {
        &self.raster_cache
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
}
