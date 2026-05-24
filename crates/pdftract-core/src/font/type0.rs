//! Type 0 composite font loader.
//!
//! This module implements loading of Type 0 composite fonts, which are used
//! for CJK text extraction. Type 0 fonts have a descendant CIDFont that
//! contains the actual font program and metrics.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::font::embedded::{EmbeddedFont, OpenTypeMetrics};
use crate::font::FontKind;
use crate::parser::object::types::{PdfDict, PdfObject};
use crate::parser::stream::{decode_stream, ExtractionOptions};

/// Result type for Type0 font operations.
pub type Type0Result<T> = Result<T, Type0Error>;

/// Errors that can occur during Type0 font loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type0Error {
    /// No DescendantFonts array found.
    NoDescendantFonts,
    /// DescendantFonts array is empty.
    EmptyDescendantFonts,
    /// Descendant is not a dictionary.
    InvalidDescendant,
    /// CIDFont subtype not supported.
    UnsupportedSubtype(String),
    /// Font descriptor missing.
    NoFontDescriptor,
    /// Font program stream missing.
    NoFontProgram,
    /// Font program decode failed.
    DecodeFailed(String),
    /// Invalid /W array format.
    InvalidWidthArray(String),
}

impl std::fmt::Display for Type0Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type0Error::NoDescendantFonts => write!(f, "no /DescendantFonts array in Type0 font"),
            Type0Error::EmptyDescendantFonts => write!(f, "empty /DescendantFonts array"),
            Type0Error::InvalidDescendant => write!(f, "descendant is not a dictionary"),
            Type0Error::UnsupportedSubtype(s) => write!(f, "unsupported CIDFont subtype: {}", s),
            Type0Error::NoFontDescriptor => write!(f, "no /FontDescriptor in CIDFont"),
            Type0Error::NoFontProgram => write!(f, "no font program stream"),
            Type0Error::DecodeFailed(msg) => write!(f, "font program decode failed: {}", msg),
            Type0Error::InvalidWidthArray(msg) => write!(f, "invalid /W array: {}", msg),
        }
    }
}

impl std::error::Error for Type0Error {}

/// CIDToGIDMap for CIDFontType2 fonts.
///
/// Maps CID (character identifier) to GID (glyph index).
#[derive(Clone, Debug)]
pub enum CIDToGIDMap {
    /// Identity mapping: GID == CID (most common for CIDFontType2).
    Identity,
    /// Custom mapping from a stream (2-byte big-endian GID values).
    Array(Box<[u16]>),
}

impl CIDToGIDMap {
    /// Get the GID for a given CID.
    pub fn get(&self, cid: u32) -> Option<u16> {
        match self {
            CIDToGIDMap::Identity => {
                // GID == CID, but GID is u16
                if cid <= u16::MAX as u32 {
                    Some(cid as u16)
                } else {
                    None
                }
            }
            CIDToGIDMap::Array(arr) => {
                // Direct index into the pre-parsed u16 array
                // GID 0 is the .notdef glyph by convention
                arr.get(cid as usize).copied().or(Some(0))
            }
        }
    }
}

/// Descendant CIDFont data.
///
/// Contains the metrics and font program from the descendant CIDFont.
#[derive(Clone)]
pub struct DescendantCIDFont {
    /// The CIDFont subtype (CIDFontType0 or CIDFontType2).
    pub subtype: FontKind,
    /// Default width (DW) in 1/1000 text units.
    pub default_width: u16,
    /// Per-CID widths (W) sparse map.
    pub widths: BTreeMap<u32, u16>,
    /// CIDToGIDMap for CIDFontType2.
    pub cid_to_gid_map: Option<CIDToGIDMap>,
    /// Embedded font program (if available).
    pub font_program: Option<Arc<EmbeddedFont>>,
    /// Diagnostics emitted during loading.
    pub diagnostics: Vec<Diagnostic>,
}

impl DescendantCIDFont {
    /// Get the width for a given CID.
    ///
    /// Returns the width from the /W array, or the default width if not present.
    pub fn get_width(&self, cid: u32) -> u16 {
        self.widths.get(&cid).copied().unwrap_or(self.default_width)
    }

    /// Get the GID for a given CID (for CIDFontType2).
    ///
    /// Returns None if this is CIDFontType0 or if the CID is out of range.
    pub fn get_gid(&self, cid: u32) -> Option<u16> {
        if let Some(ref map) = self.cid_to_gid_map {
            map.get(cid)
        } else if self.subtype == FontKind::CIDFontType2 {
            // Default to Identity for CIDFontType2
            CIDToGIDMap::Identity.get(cid)
        } else {
            None
        }
    }
}

/// Type 0 composite font.
///
/// Contains the parent Type0 font data and the descendant CIDFont.
#[derive(Clone)]
pub struct Type0Font {
    /// The descendant CIDFont.
    pub descendant: DescendantCIDFont,
    /// BaseFont name from the Type0 font dict.
    pub base_font: Arc<str>,
    /// Encoding CMap name (deferred to Phase 2.3 CJK).
    pub encoding_name: Option<Arc<str>>,
}

impl Type0Font {
    /// Load a Type 0 composite font from its dictionary.
    ///
    /// # Parameters
    ///
    /// - `font_dict`: The Type0 font dictionary from the resource dictionary
    /// - `source`: The PDF source to read font program streams from
    /// - `opts`: Extraction options (for stream decoding limits)
    /// - `doc_counter`: Cumulative decompressed bytes counter
    ///
    /// # Returns
    ///
    /// A `Type0Result` containing the `Type0Font` or a `Type0Error`.
    pub fn load(
        font_dict: &PdfDict,
        source: &dyn crate::parser::stream::PdfSource,
        opts: &ExtractionOptions,
        doc_counter: &mut u64,
    ) -> Type0Result<Self> {
        let mut diagnostics = Vec::new();

        // Get BaseFont
        let base_font = font_dict
            .get("/BaseFont")
            .and_then(|obj| obj.as_name())
            .unwrap_or("");

        // Get Encoding (deferred to Phase 2.3 CJK)
        let encoding_name = font_dict
            .get("/Encoding")
            .and_then(|obj| obj.as_name())
            .map(|s| Arc::<str>::from(s));

        // Get /DescendantFonts array
        let descendants = match font_dict.get("/DescendantFonts") {
            Some(PdfObject::Array(arr)) => arr.as_ref(),
            Some(PdfObject::Ref(_)) => {
                // Indirect reference - would need resolution
                return Err(Type0Error::NoDescendantFonts);
            }
            _ => return Err(Type0Error::NoDescendantFonts),
        };

        if descendants.is_empty() {
            return Err(Type0Error::EmptyDescendantFonts);
        }

        // Get the first descendant (Type0 fonts typically have one)
        let cidfont_dict = match &descendants[0] {
            PdfObject::Dict(d) => d.as_ref(),
            PdfObject::Ref(_) => {
                // Indirect reference - would need resolution
                return Err(Type0Error::InvalidDescendant);
            }
            _ => return Err(Type0Error::InvalidDescendant),
        };

        // Get CIDFont subtype
        let subtype_name = cidfont_dict
            .get("/Subtype")
            .and_then(|obj| obj.as_name())
            .unwrap_or("");

        let subtype_clean = if subtype_name.starts_with('/') {
            &subtype_name[1..]
        } else {
            subtype_name
        };

        let subtype = match subtype_clean {
            "CIDFontType0" => FontKind::CIDFontType0,
            "CIDFontType2" => FontKind::CIDFontType2,
            _ => {
                return Err(Type0Error::UnsupportedSubtype(subtype_clean.into()));
            }
        };

        // Get DW (default width), default 1000
        let default_width = cidfont_dict
            .get("/DW")
            .and_then(|obj| obj.as_int())
            .map(|i| i as u16)
            .unwrap_or(1000);

        // Parse /W array
        let widths = Self::parse_w_array(cidfont_dict)?;

        // Load CIDToGIDMap for CIDFontType2
        let cid_to_gid_map = if subtype == FontKind::CIDFontType2 {
            Some(Self::load_cid_to_gid_map(
                cidfont_dict,
                source,
                opts,
                doc_counter,
                &mut diagnostics,
            )?)
        } else {
            None
        };

        // Load the embedded font program from the descendant's FontDescriptor
        let font_program = match Self::load_font_program(
            cidfont_dict,
            subtype,
            source,
            opts,
            doc_counter,
            &mut diagnostics,
        ) {
            Ok(program) => Some(program),
            Err(e) => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::FontParseFailed,
                    format!("Failed to load font program: {}", e),
                ));
                None
            }
        };

        Ok(Self {
            descendant: DescendantCIDFont {
                subtype,
                default_width,
                widths,
                cid_to_gid_map,
                font_program,
                diagnostics,
            },
            base_font: Arc::<str>::from(base_font),
            encoding_name,
        })
    }

    /// Parse the /W array from a CIDFont dictionary.
    ///
    /// The /W array has two formats:
    /// 1. Per-CID form: `[c [w1 w2 ...]]` - maps starting CID c to sequential widths
    /// 2. Range form: `[cfirst clast w]` - maps all CIDs in [cfirst, clast] to width w
    fn parse_w_array(cidfont_dict: &PdfDict) -> Type0Result<BTreeMap<u32, u16>> {
        let mut widths = BTreeMap::new();

        let w_array = match cidfont_dict.get("/W") {
            Some(PdfObject::Array(arr)) => arr.as_ref(),
            Some(PdfObject::Ref(_)) => {
                // Indirect reference - not supported yet
                return Ok(widths);
            }
            _ => return Ok(widths), // No /W array is valid
        };

        let mut i = 0;
        while i < w_array.len() {
            // Format 1: [c [w1 w2 ...]]
            // Need at least: integer (start CID) + array (widths)
            if i + 1 >= w_array.len() {
                break;
            }

            let start_cid = match w_array[i].as_int() {
                Some(v) if v >= 0 => v as u32,
                _ => {
                    i += 1;
                    continue;
                }
            };

            // Check if next element is an array (format 1) or integer (format 2)
            match &w_array[i + 1] {
                PdfObject::Array(width_arr) => {
                    // Format 1: [c [w1 w2 w3 ...]]
                    // Map sequential CIDs starting from start_cid to each width
                    for (j, width_obj) in width_arr.iter().enumerate() {
                        if let Some(width) = width_obj.as_int() {
                            if width >= 0 && width <= u16::MAX as i64 {
                                widths.insert(start_cid + j as u32, width as u16);
                            }
                        }
                    }
                    i += 2;
                }
                _ => {
                    // Format 2: [cfirst clast w]
                    // Need at least 3 elements: start, end, width
                    if i + 2 >= w_array.len() {
                        break;
                    }

                    let end_cid = match w_array[i + 1].as_int() {
                        Some(v) if v >= 0 => v as u32,
                        _ => {
                            i += 1;
                            continue;
                        }
                    };

                    let width = match w_array[i + 2].as_int() {
                        Some(v) if v >= 0 && v <= u16::MAX as i64 => v as u16,
                        _ => {
                            i += 1;
                            continue;
                        }
                    };

                    // Map all CIDs in [start_cid, end_cid] to the same width
                    for cid in start_cid..=end_cid {
                        widths.insert(cid, width);
                    }
                    i += 3;
                }
            }
        }

        Ok(widths)
    }

    /// Load the CIDToGIDMap from a CIDFontType2 dictionary.
    ///
    /// Returns the appropriate CIDToGIDMap variant.
    ///
    /// Emits `CIDTOGIDMAP_TRUNCATED` diagnostic if the stream has an odd byte count.
    fn load_cid_to_gid_map(
        cidfont_dict: &PdfDict,
        source: &dyn crate::parser::stream::PdfSource,
        opts: &ExtractionOptions,
        doc_counter: &mut u64,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Type0Result<CIDToGIDMap> {
        match cidfont_dict.get("/CIDToGIDMap") {
            Some(PdfObject::Name(name)) => {
                let name_str: &str = name.as_ref();
                let stripped = if name_str.starts_with('/') {
                    &name_str[1..]
                } else {
                    name_str
                };

                if stripped == "Identity" {
                    Ok(CIDToGIDMap::Identity)
                } else {
                    // Unknown name - treat as Identity but warn
                    Ok(CIDToGIDMap::Identity)
                }
            }
            Some(PdfObject::Stream(stream)) => {
                // Decode the stream as 2-byte big-endian GID values
                let data = decode_stream(stream, source, opts, doc_counter);
                if data.is_empty() {
                    Ok(CIDToGIDMap::Identity)
                } else {
                    // Check for odd byte count
                    let truncated = data.len() % 2 != 0;
                    if truncated {
                        diagnostics.push(Diagnostic::with_static_no_offset(
                            DiagCode::FontCidtogidmapTruncated,
                            "CIDToGIDMap stream has odd byte count; trailing byte discarded",
                        ));
                    }

                    // Parse into u16 array (big-endian)
                    let len = data.len() / 2;
                    let mut arr = Vec::with_capacity(len);
                    for i in 0..len {
                        let gid = u16::from_be_bytes([data[i * 2], data[i * 2 + 1]]);
                        arr.push(gid);
                    }
                    Ok(CIDToGIDMap::Array(arr.into_boxed_slice()))
                }
            }
            Some(PdfObject::Ref(_)) => {
                // Indirect reference - not supported yet, default to Identity
                Ok(CIDToGIDMap::Identity)
            }
            _ => Ok(CIDToGIDMap::Identity), // Default to Identity
        }
    }

    /// Load the embedded font program from the CIDFont's FontDescriptor.
    ///
    /// This constructs a minimal font dict that includes the FontDescriptor
    /// from the CIDFont, then delegates to EmbeddedFont::load.
    fn load_font_program(
        cidfont_dict: &PdfDict,
        subtype: FontKind,
        source: &dyn crate::parser::stream::PdfSource,
        opts: &ExtractionOptions,
        doc_counter: &mut u64,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Type0Result<Arc<EmbeddedFont>> {
        // Check if FontDescriptor exists
        match cidfont_dict.get("/FontDescriptor") {
            Some(PdfObject::Dict(_)) => {
                // FontDescriptor is present - we can try to load the font
                // Build a minimal font dict for EmbeddedFont::load
                let mut font_dict = PdfDict::new();
                font_dict.insert(
                    crate::parser::object::types::intern("/Subtype"),
                    match subtype {
                        FontKind::CIDFontType0 => {
                            PdfObject::Name(crate::parser::object::types::intern("/CIDFontType0"))
                        }
                        FontKind::CIDFontType2 => {
                            PdfObject::Name(crate::parser::object::types::intern("/CIDFontType2"))
                        }
                        _ => return Err(Type0Error::UnsupportedSubtype(format!("{:?}", subtype))),
                    },
                );
                font_dict.insert(
                    crate::parser::object::types::intern("/BaseFont"),
                    PdfObject::Name(crate::parser::object::types::intern("CIDFont")),
                );
                // Copy the FontDescriptor reference
                font_dict.insert(
                    crate::parser::object::types::intern("/FontDescriptor"),
                    cidfont_dict.get("/FontDescriptor").unwrap().clone(),
                );

                // Use EmbeddedFont::load which handles all the details
                match EmbeddedFont::load(&font_dict, source, opts, doc_counter) {
                    Ok(font) => Ok(Arc::new(font)),
                    Err(e) => {
                        diagnostics.push(Diagnostic::with_dynamic_no_offset(
                            DiagCode::FontParseFailed,
                            format!("Font program load failed: {}", e),
                        ));
                        Err(Type0Error::DecodeFailed(e.to_string()))
                    }
                }
            }
            Some(PdfObject::Ref(_)) => {
                // Indirect reference - would need resolution
                diagnostics.push(Diagnostic::with_static_no_offset(
                    DiagCode::FontParseFailed,
                    "FontDescriptor is indirect reference - not supported",
                ));
                Err(Type0Error::NoFontDescriptor)
            }
            _ => Err(Type0Error::NoFontDescriptor),
        }
    }

    /// Get the width for a given CID.
    pub fn get_width(&self, cid: u32) -> u16 {
        self.descendant.get_width(cid)
    }

    /// Get the GID for a given CID (for CIDFontType2).
    pub fn get_gid(&self, cid: u32) -> Option<u16> {
        self.descendant.get_gid(cid)
    }

    /// Get the embedded font program if available.
    pub fn font_program(&self) -> Option<&Arc<EmbeddedFont>> {
        self.descendant.font_program.as_ref()
    }

    /// Get diagnostics emitted during loading.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.descendant.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::types::intern;
    use crate::parser::stream::MemorySource;

    #[test]
    fn test_cid_to_gid_map_identity() {
        let map = CIDToGIDMap::Identity;

        assert_eq!(map.get(0), Some(0));
        assert_eq!(map.get(100), Some(100));
        assert_eq!(map.get(65535), Some(65535));
        assert_eq!(map.get(65536), None); // Out of u16 range
    }

    #[test]
    fn test_cid_to_gid_map_array() {
        // Create a simple custom map: [0, 1, 2, 3]
        // Maps CID 0 -> GID 0, CID 1 -> GID 1, etc.
        let arr = vec![0u16, 1, 2, 3].into_boxed_slice();
        let map = CIDToGIDMap::Array(arr);

        assert_eq!(map.get(0), Some(0));
        assert_eq!(map.get(1), Some(1));
        assert_eq!(map.get(2), Some(2));
        assert_eq!(map.get(3), Some(3));
        // Out of range returns GID 0 (notdef), not None
        assert_eq!(map.get(4), Some(0));
    }

    #[test]
    fn test_cid_to_gid_map_array_big_endian() {
        // Test that parsed values are correct: CID 5 should map to GID 0x1234
        let mut arr = vec![0u16; 6];
        arr[5] = 0x1234;
        let map = CIDToGIDMap::Array(arr.into_boxed_slice());

        assert_eq!(map.get(5), Some(0x1234));
    }

    #[test]
    fn test_parse_w_array_format1() {
        // Format 1: [10 [500 600 700]]
        // Maps: CID 10 -> 500, CID 11 -> 600, CID 12 -> 700
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(
            intern("/W"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(10),
                PdfObject::Array(Box::new(vec![
                    PdfObject::Integer(500),
                    PdfObject::Integer(600),
                    PdfObject::Integer(700),
                ])),
            ])),
        );

        let widths = Type0Font::parse_w_array(&cidfont_dict).unwrap();

        assert_eq!(widths.get(&10), Some(&500));
        assert_eq!(widths.get(&11), Some(&600));
        assert_eq!(widths.get(&12), Some(&700));
        assert_eq!(widths.get(&13), None);
    }

    #[test]
    fn test_parse_w_array_format2() {
        // Format 2: [100 200 800]
        // Maps: CIDs 100..=200 all -> 800
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(
            intern("/W"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(100),
                PdfObject::Integer(200),
                PdfObject::Integer(800),
            ])),
        );

        let widths = Type0Font::parse_w_array(&cidfont_dict).unwrap();

        assert_eq!(widths.get(&100), Some(&800));
        assert_eq!(widths.get(&150), Some(&800));
        assert_eq!(widths.get(&200), Some(&800));
        assert_eq!(widths.get(&99), None);
        assert_eq!(widths.get(&201), None);
    }

    #[test]
    fn test_parse_w_array_mixed() {
        // Mixed: [10 [500 600]] [100 200 800]
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(
            intern("/W"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(10),
                PdfObject::Array(Box::new(vec![
                    PdfObject::Integer(500),
                    PdfObject::Integer(600),
                ])),
                PdfObject::Integer(100),
                PdfObject::Integer(200),
                PdfObject::Integer(800),
            ])),
        );

        let widths = Type0Font::parse_w_array(&cidfont_dict).unwrap();

        assert_eq!(widths.get(&10), Some(&500));
        assert_eq!(widths.get(&11), Some(&600));
        assert_eq!(widths.get(&12), None);
        assert_eq!(widths.get(&100), Some(&800));
        assert_eq!(widths.get(&200), Some(&800));
    }

    #[test]
    fn test_parse_w_array_empty_subarray() {
        // Edge case: [10 []]
        // Empty subarray should be ignored
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(
            intern("/W"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(10),
                PdfObject::Array(Box::new(vec![])),
            ])),
        );

        let widths = Type0Font::parse_w_array(&cidfont_dict).unwrap();

        assert_eq!(widths.get(&10), None);
    }

    #[test]
    fn test_parse_w_array_no_w() {
        // No /W array is valid
        let cidfont_dict = PdfDict::new();

        let widths = Type0Font::parse_w_array(&cidfont_dict).unwrap();

        assert!(widths.is_empty());
    }

    #[test]
    fn test_descendant_get_width_default() {
        // Default width (DW) is 1000
        let descendant = DescendantCIDFont {
            subtype: FontKind::CIDFontType2,
            default_width: 1000,
            widths: BTreeMap::new(),
            cid_to_gid_map: None,
            font_program: None,
            diagnostics: Vec::new(),
        };

        assert_eq!(descendant.get_width(0), 1000);
        assert_eq!(descendant.get_width(100), 1000);
    }

    #[test]
    fn test_descendant_get_width_from_w() {
        // CID 10 -> 500, others -> DW (1000)
        let mut widths = BTreeMap::new();
        widths.insert(10, 500);

        let descendant = DescendantCIDFont {
            subtype: FontKind::CIDFontType2,
            default_width: 1000,
            widths,
            cid_to_gid_map: None,
            font_program: None,
            diagnostics: Vec::new(),
        };

        assert_eq!(descendant.get_width(10), 500);
        assert_eq!(descendant.get_width(11), 1000);
    }

    #[test]
    fn test_descendant_get_gid_identity() {
        let descendant = DescendantCIDFont {
            subtype: FontKind::CIDFontType2,
            default_width: 1000,
            widths: BTreeMap::new(),
            cid_to_gid_map: Some(CIDToGIDMap::Identity),
            font_program: None,
            diagnostics: Vec::new(),
        };

        assert_eq!(descendant.get_gid(0), Some(0));
        assert_eq!(descendant.get_gid(100), Some(100));
        assert_eq!(descendant.get_gid(65535), Some(65535));
        assert_eq!(descendant.get_gid(65536), None);
    }

    #[test]
    fn test_descendant_get_gid_cidfonttype0() {
        // CIDFontType0 doesn't have GIDs
        let descendant = DescendantCIDFont {
            subtype: FontKind::CIDFontType0,
            default_width: 1000,
            widths: BTreeMap::new(),
            cid_to_gid_map: None,
            font_program: None,
            diagnostics: Vec::new(),
        };

        assert_eq!(descendant.get_gid(0), None);
        assert_eq!(descendant.get_gid(100), None);
    }

    #[test]
    fn test_load_type0_font_minimal() {
        // Create a minimal Type0 font dict
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/CIDFontType2")));
        cidfont_dict.insert(intern("/BaseFont"), PdfObject::Name(intern("CIDFont")));

        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));
        font_dict.insert(intern("/BaseFont"), PdfObject::Name(intern("Type0Font")));
        font_dict.insert(
            intern("/DescendantFonts"),
            PdfObject::Array(Box::new(vec![PdfObject::Dict(Box::new(cidfont_dict))])),
        );

        let source = MemorySource::new(vec![]);
        let opts = ExtractionOptions::default();
        let mut counter = 0;

        let result = Type0Font::load(&font_dict, &source, &opts, &mut counter);

        // Should succeed without font program
        assert!(result.is_ok());
        let font = result.unwrap();
        assert_eq!(font.base_font.as_ref(), "Type0Font");
        assert_eq!(font.descendant.default_width, 1000);
    }

    #[test]
    fn test_load_type0_font_with_dw() {
        // Test custom DW
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/CIDFontType2")));
        cidfont_dict.insert(intern("/DW"), PdfObject::Integer(500));

        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));
        font_dict.insert(
            intern("/DescendantFonts"),
            PdfObject::Array(Box::new(vec![PdfObject::Dict(Box::new(cidfont_dict))])),
        );

        let source = MemorySource::new(vec![]);
        let opts = ExtractionOptions::default();
        let mut counter = 0;

        let result = Type0Font::load(&font_dict, &source, &opts, &mut counter);

        assert!(result.is_ok());
        let font = result.unwrap();
        assert_eq!(font.descendant.default_width, 500);
    }

    #[test]
    fn test_load_type0_font_with_w() {
        // Test /W array parsing
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/CIDFontType2")));
        cidfont_dict.insert(
            intern("/W"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(10),
                PdfObject::Array(Box::new(vec![
                    PdfObject::Integer(500),
                    PdfObject::Integer(600),
                ])),
            ])),
        );

        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));
        font_dict.insert(
            intern("/DescendantFonts"),
            PdfObject::Array(Box::new(vec![PdfObject::Dict(Box::new(cidfont_dict))])),
        );

        let source = MemorySource::new(vec![]);
        let opts = ExtractionOptions::default();
        let mut counter = 0;

        let result = Type0Font::load(&font_dict, &source, &opts, &mut counter);

        assert!(result.is_ok());
        let font = result.unwrap();
        assert_eq!(font.get_width(10), 500);
        assert_eq!(font.get_width(11), 600);
        assert_eq!(font.get_width(12), 1000); // Falls back to DW
    }

    #[test]
    fn test_load_type0_font_cidfonttype0() {
        // Test CIDFontType0 descendant
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/CIDFontType0")));

        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));
        font_dict.insert(
            intern("/DescendantFonts"),
            PdfObject::Array(Box::new(vec![PdfObject::Dict(Box::new(cidfont_dict))])),
        );

        let source = MemorySource::new(vec![]);
        let opts = ExtractionOptions::default();
        let mut counter = 0;

        let result = Type0Font::load(&font_dict, &source, &opts, &mut counter);

        assert!(result.is_ok());
        let font = result.unwrap();
        assert_eq!(font.descendant.subtype, FontKind::CIDFontType0);
    }

    #[test]
    fn test_load_type0_font_no_descendants() {
        // Missing /DescendantFonts
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));

        let source = MemorySource::new(vec![]);
        let opts = ExtractionOptions::default();
        let mut counter = 0;

        let result = Type0Font::load(&font_dict, &source, &opts, &mut counter);

        assert!(matches!(result, Err(Type0Error::NoDescendantFonts)));
    }

    #[test]
    fn test_load_type0_font_empty_descendants() {
        // Empty /DescendantFonts array
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));
        font_dict.insert(
            intern("/DescendantFonts"),
            PdfObject::Array(Box::new(vec![])),
        );

        let source = MemorySource::new(vec![]);
        let opts = ExtractionOptions::default();
        let mut counter = 0;

        let result = Type0Font::load(&font_dict, &source, &opts, &mut counter);

        assert!(matches!(result, Err(Type0Error::EmptyDescendantFonts)));
    }

    #[test]
    fn test_acceptance_type0_with_cidfonttype2() {
        // A Type0 font with CIDFontType2 descendant loads
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/CIDFontType2")));
        // Widths: [10 [500 600]] means CID 10 -> 500, CID 11 -> 600
        cidfont_dict.insert(
            intern("/W"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(10),
                PdfObject::Array(Box::new(vec![
                    PdfObject::Integer(500),
                    PdfObject::Integer(600),
                ])),
            ])),
        );

        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));
        font_dict.insert(
            intern("/DescendantFonts"),
            PdfObject::Array(Box::new(vec![PdfObject::Dict(Box::new(cidfont_dict))])),
        );

        let source = MemorySource::new(vec![]);
        let opts = ExtractionOptions::default();
        let mut counter = 0;

        let result = Type0Font::load(&font_dict, &source, &opts, &mut counter);
        assert!(result.is_ok());

        let font = result.unwrap();
        // CID 10 -> 500
        assert_eq!(font.get_width(10), 500);
        // CID 11 -> 600
        assert_eq!(font.get_width(11), 600);
    }

    #[test]
    fn test_acceptance_range_form() {
        // Range form [100 200 800] means CIDs 100..=200 all -> 800
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/CIDFontType2")));
        cidfont_dict.insert(
            intern("/W"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(100),
                PdfObject::Integer(200),
                PdfObject::Integer(800),
            ])),
        );

        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));
        font_dict.insert(
            intern("/DescendantFonts"),
            PdfObject::Array(Box::new(vec![PdfObject::Dict(Box::new(cidfont_dict))])),
        );

        let source = MemorySource::new(vec![]);
        let opts = ExtractionOptions::default();
        let mut counter = 0;

        let result = Type0Font::load(&font_dict, &source, &opts, &mut counter);
        assert!(result.is_ok());

        let font = result.unwrap();
        // CIDs 100..=200 all map to 800
        assert_eq!(font.get_width(100), 800);
        assert_eq!(font.get_width(150), 800);
        assert_eq!(font.get_width(200), 800);
    }

    #[test]
    fn test_acceptance_missing_cid_fallback() {
        // Missing CID falls back to DW (default 1000)
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/CIDFontType2")));
        // No /W array, so all CIDs fall back to DW

        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));
        font_dict.insert(
            intern("/DescendantFonts"),
            PdfObject::Array(Box::new(vec![PdfObject::Dict(Box::new(cidfont_dict))])),
        );

        let source = MemorySource::new(vec![]);
        let opts = ExtractionOptions::default();
        let mut counter = 0;

        let result = Type0Font::load(&font_dict, &source, &opts, &mut counter);
        assert!(result.is_ok());

        let font = result.unwrap();
        // Any CID should fall back to DW (1000)
        assert_eq!(font.get_width(0), 1000);
        assert_eq!(font.get_width(999), 1000);
        assert_eq!(font.get_width(50000), 1000);
    }

    #[test]
    fn test_cid_to_gid_map_from_stream() {
        // Test loading CIDToGIDMap from a stream
        // The stream data: [0x00, 0x00, 0x00, 0x05, 0x00, 0x0A]
        // Maps: CID 0 -> GID 0, CID 1 -> GID 5, CID 2 -> GID 10
        let stream_data = vec![0x00u8, 0x00, 0x00, 0x05, 0x00, 0x0A];

        // Create a MemorySource with the stream data at offset 0
        let mut full_data = vec![0u8; 1000]; // Reserve space for PDF-like structure
        full_data[0..stream_data.len()].copy_from_slice(&stream_data);
        let source = MemorySource::new(full_data);

        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/CIDFontType2")));
        cidfont_dict.insert(intern("/BaseFont"), PdfObject::Name(intern("CIDFont")));
        cidfont_dict.insert(
            intern("/CIDToGIDMap"),
            PdfObject::Stream(Box::new(crate::parser::object::types::PdfStream {
                dict: PdfDict::new(),
                offset: 0,
                len_hint: Some(6),
            })),
        );

        // Wrap in a Type0 font dict
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));
        font_dict.insert(intern("/BaseFont"), PdfObject::Name(intern("Type0Font")));
        font_dict.insert(
            intern("/DescendantFonts"),
            PdfObject::Array(Box::new(vec![PdfObject::Dict(Box::new(cidfont_dict))])),
        );

        let opts = ExtractionOptions::default();
        let mut counter = 0;

        let result = Type0Font::load(&font_dict, &source, &opts, &mut counter);

        // The load should succeed (even if FontDescriptor is missing, we handle it gracefully)
        assert!(result.is_ok());
        let font = result.unwrap();
        // The CIDToGIDMap should be loaded (as Array since stream decode succeeds)
        assert!(font.descendant.cid_to_gid_map.is_some());

        // Verify the array map works
        if let Some(CIDToGIDMap::Array(arr)) = &font.descendant.cid_to_gid_map {
            assert_eq!(arr.len(), 3);
            // Verify the values are correct: [0, 5, 10]
            assert_eq!(arr[0], 0);
            assert_eq!(arr[1], 5);
            assert_eq!(arr[2], 10);

            // Verify the get method works
            assert_eq!(font.descendant.get_gid(0), Some(0));
            assert_eq!(font.descendant.get_gid(1), Some(5));
            assert_eq!(font.descendant.get_gid(2), Some(10));
        }
    }

    #[test]
    fn test_cid_to_gid_map_truncated() {
        // Test loading CIDToGIDMap from a stream with odd byte count
        // The stream data: [0x00, 0x00, 0x00, 0x05, 0x00] (5 bytes - odd!)
        // Should emit CIDTOGIDMAP_TRUNCATED and truncate to 2 GIDs: [0, 5]
        let stream_data = vec![0x00u8, 0x00, 0x00, 0x05, 0x00];

        // Create a MemorySource with the stream data at offset 0
        let mut full_data = vec![0u8; 1000];
        full_data[0..stream_data.len()].copy_from_slice(&stream_data);
        let source = MemorySource::new(full_data);

        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/CIDFontType2")));
        cidfont_dict.insert(intern("/BaseFont"), PdfObject::Name(intern("CIDFont")));
        cidfont_dict.insert(
            intern("/CIDToGIDMap"),
            PdfObject::Stream(Box::new(crate::parser::object::types::PdfStream {
                dict: PdfDict::new(),
                offset: 0,
                len_hint: Some(5),
            })),
        );

        // Wrap in a Type0 font dict
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));
        font_dict.insert(intern("/BaseFont"), PdfObject::Name(intern("Type0Font")));
        font_dict.insert(
            intern("/DescendantFonts"),
            PdfObject::Array(Box::new(vec![PdfObject::Dict(Box::new(cidfont_dict))])),
        );

        let opts = ExtractionOptions::default();
        let mut counter = 0;

        let result = Type0Font::load(&font_dict, &source, &opts, &mut counter);

        // The load should succeed
        assert!(result.is_ok());
        let font = result.unwrap();

        // Check that the CIDTOGIDMAP_TRUNCATED diagnostic was emitted
        let diagnostics = font.diagnostics();
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagCode::FontCidtogidmapTruncated));

        // Verify the array has 2 elements (5 bytes / 2 = 2 GIDs, trailing byte discarded)
        if let Some(CIDToGIDMap::Array(arr)) = &font.descendant.cid_to_gid_map {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0], 0);
            assert_eq!(arr[1], 5);
        }
    }

    #[test]
    fn test_cid_to_gid_map_out_of_range() {
        // Test that out-of-range CID returns GID 0 (notdef), not None
        let arr = vec![0u16, 5, 10].into_boxed_slice();
        let map = CIDToGIDMap::Array(arr);

        // Valid CIDs
        assert_eq!(map.get(0), Some(0));
        assert_eq!(map.get(1), Some(5));
        assert_eq!(map.get(2), Some(10));

        // Out of range CID should return GID 0 (notdef), not None
        assert_eq!(map.get(3), Some(0));
        assert_eq!(map.get(100), Some(0));
        assert_eq!(map.get(65535), Some(0));
    }

    #[test]
    fn test_parse_w_array_high_cid_values() {
        // Test that high CID values (e.g., 50000+) work correctly
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(
            intern("/W"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(50000),
                PdfObject::Integer(50002),
                PdfObject::Integer(900),
            ])),
        );

        let widths = Type0Font::parse_w_array(&cidfont_dict).unwrap();

        assert_eq!(widths.get(&50000), Some(&900));
        assert_eq!(widths.get(&50001), Some(&900));
        assert_eq!(widths.get(&50002), Some(&900));
        assert_eq!(widths.get(&49999), None);
        assert_eq!(widths.get(&50003), None);
    }
}
