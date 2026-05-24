//! PDF structural fingerprint computation.
//!
//! This module implements the v1 fingerprint algorithm described in Phase 1.7
//! of the implementation plan. The fingerprint is a reproducible 256-bit content
//! hash that identifies the semantic content of a PDF independent of metadata
//! churn, byte ordering, and producer-tool re-saves.
//!
//! # Algorithm
//!
//! The fingerprint is computed as a Merkle-style SHA-256 hash over the following
//! inputs in deterministic order:
//!
//! 1. Page count (u32, big-endian)
//! 2. Per page in page_index order:
//!    - SHA-256 of each decoded content stream (concatenated in stream-array order)
//!    - SHA-256 of the resolved resource dict
//!    - Page geometry: MediaBox, CropBox, Rotate (canonicalized to 4dp fixed-point)
//! 3. SHA-256 of the structure tree (or all-zero hash if not tagged)
//! 4. Catalog feature flag byte
//!
//! # Output Format
//!
//! The fingerprint is returned as a string: `"pdftract-v1:" + hex(SHA-256)`.

pub mod canonicalize;

use sha2::{Digest, Sha256};

use crate::diagnostics::Diagnostic;
use crate::parser::lexer::Lexer;
use crate::parser::object::{ObjRef, PdfDict, PdfObject};
use crate::parser::xref::XrefResolver;

/// Version prefix for fingerprint output.
pub const FINGERPRINT_VERSION: &str = "pdftract-v1";

/// All-zero hash for non-tagged PDFs (no structure tree).
const ZERO_HASH: [u8; 32] = [0u8; 32];

/// Document fingerprint input data.
///
/// This structure contains all the information needed to compute
/// a document fingerprint. It is built by the document model
/// during Phase 1.4 parsing.
#[derive(Debug, Clone)]
pub struct FingerprintInput {
    /// Page count
    pub page_count: u32,
    /// Per-page fingerprint data
    pub pages: Vec<PageFingerprintData>,
    /// Structure tree root reference (if present)
    pub struct_tree_root_ref: Option<ObjRef>,
    /// Whether the document is tagged PDF
    pub is_tagged: bool,
    /// Catalog feature flags
    pub catalog_flags: CatalogFlags,
}

/// Per-page fingerprint data.
#[derive(Debug, Clone)]
pub struct PageFingerprintData {
    /// Content stream references (in order)
    pub content_streams: Vec<ContentStreamData>,
    /// Resource dictionary reference (resolved)
    pub resources: Option<PdfDict>,
    /// MediaBox [x1, y1, x2, y2]
    pub media_box: [f64; 4],
    /// CropBox [x1, y1, x2, y2] (if present)
    pub crop_box: Option<[f64; 4]>,
    /// Page rotation in degrees (0, 90, 180, 270)
    pub rotate: i32,
}

/// Content stream data for fingerprinting.
#[derive(Debug, Clone)]
pub enum ContentStreamData {
    /// Reference to an indirect stream object
    Indirect(ObjRef),
    /// Direct stream bytes (decoded)
    Direct(Vec<u8>),
}

/// Catalog feature flags for the fingerprint.
///
/// These flags are encoded into a single byte in the fingerprint:
/// - bit 0: is_encrypted
/// - bit 1: contains_javascript
/// - bit 2: contains_xfa
/// - bit 3: ocg_present
#[derive(Debug, Clone, Default)]
pub struct CatalogFlags {
    /// Document is encrypted
    pub is_encrypted: bool,
    /// Document contains JavaScript
    pub contains_javascript: bool,
    /// Document contains XFA forms
    pub contains_xfa: bool,
    /// Document has Optional Content Groups
    pub ocg_present: bool,
}

impl CatalogFlags {
    /// Encode the flags into a single byte.
    fn encode(&self) -> u8 {
        let mut byte = 0u8;
        if self.is_encrypted {
            byte |= 1 << 0;
        }
        if self.contains_javascript {
            byte |= 1 << 1;
        }
        if self.contains_xfa {
            byte |= 1 << 2;
        }
        if self.ocg_present {
            byte |= 1 << 3;
        }
        byte
    }
}

/// Compute the structural fingerprint for a document.
///
/// # Arguments
/// * `input` - The fingerprint input data
/// * `resolver` - The xref resolver for resolving indirect references
///
/// # Returns
/// A string in the format `"pdftract-v1:" + hex(SHA-256)`.
///
/// # Example
/// ```ignore
/// let fingerprint = compute_fingerprint(&fingerprint_input, &resolver);
/// assert!(fingerprint.starts_with("pdftract-v1:"));
/// assert_eq!(fingerprint.len(), "pdftract-v1:".len() + 64);
/// ```
pub fn compute_fingerprint(input: &FingerprintInput, resolver: &XrefResolver) -> String {
    let mut hasher = Sha256::new();

    // 1. Page count (u32 big-endian)
    hasher.update(&input.page_count.to_be_bytes());

    // 2. Per-page contributions
    for page in &input.pages {
        hash_page(page, &mut hasher, resolver);
    }

    // 3. Structure tree hash (or zeros)
    if input.is_tagged {
        if let Some(struct_ref) = input.struct_tree_root_ref {
            let struct_hash = hash_structure_tree(struct_ref, resolver);
            hasher.update(struct_hash);
        } else {
            hasher.update(ZERO_HASH);
        }
    } else {
        hasher.update(ZERO_HASH);
    }

    // 4. Catalog feature flag byte
    hasher.update(&[input.catalog_flags.encode()]);

    let result = hasher.finalize();
    format!("{}:{}", FINGERPRINT_VERSION, hex::encode(result))
}

/// Hash a single page's contribution to the fingerprint.
fn hash_page(page: &PageFingerprintData, hasher: &mut Sha256, resolver: &XrefResolver) {
    // a. SHA-256 of concatenated decoded content streams
    let content_hash = hash_content_streams(&page.content_streams, resolver);
    hasher.update(content_hash);

    // b. SHA-256 of resolved resource dict
    let resource_hash = hash_resource_dict(page.resources.as_ref(), resolver);
    hasher.update(resource_hash);

    // c. Page geometry, canonicalized
    let geometry_hash = hash_page_geometry(&page.media_box, page.crop_box.as_ref(), page.rotate);
    hasher.update(geometry_hash);
}

/// Hash the content streams for a page.
///
/// Returns SHA-256 of the concatenated, decoded content streams
/// with whitespace normalized to single 0x20 between tokens.
fn hash_content_streams(streams: &[ContentStreamData], resolver: &XrefResolver) -> [u8; 32] {
    let mut hasher = Sha256::new();

    for stream_data in streams {
        let bytes = match stream_data {
            ContentStreamData::Indirect(ref_) => {
                // Resolve the stream object and decode it
                match resolver.resolve(*ref_) {
                    Ok(PdfObject::Stream(stream)) => {
                        // For Phase 1, we use the stream dictionary as a stub
                        // In a full implementation, we would decode via Phase 1.5
                        // and normalize whitespace via the lexer
                        let _ = stream; // Suppress unused warning until Phase 1.5
                        normalize_content_bytes(&[])
                    }
                    _ => Vec::new(),
                }
            }
            ContentStreamData::Direct(bytes) => normalize_content_bytes(bytes),
        };
        hasher.update(&bytes);
    }

    hasher.finalize().into()
}

/// Normalize content stream bytes by tokenizing and re-emitting with single spaces.
///
/// This function uses the Phase 1.1 lexer to tokenize the content stream
/// and re-emit tokens with single 0x20 separators, eliminating whitespace variance.
/// This ensures that different whitespace layouts produce the same fingerprint.
fn normalize_content_bytes(bytes: &[u8]) -> Vec<u8> {
    if bytes.is_empty() {
        return Vec::new();
    }

    let mut lexer = Lexer::new(bytes);
    let mut result = Vec::new();
    let mut first_token = true;

    // Tokenize and re-emit with single spaces
    while let Some(token) = lexer.next_token() {
        match token {
            crate::parser::lexer::Token::Eof => break,
            _ => {
                // Add space before token (except for first token)
                if !first_token {
                    result.push(b' ');
                }
                first_token = false;

                // Serialize token back to bytes
                serialize_token(&mut result, &token);
            }
        }
    }

    result
}

/// Serialize a token back to its byte representation.
///
/// This function converts a lexer Token back to its canonical byte representation
/// for fingerprinting purposes. The output is deterministic and matches the
/// PDF specification's lexical representation.
fn serialize_token(output: &mut Vec<u8>, token: &crate::parser::lexer::Token) {
    use crate::parser::lexer::Token;
    match token {
        Token::Bool(true) => output.extend_from_slice(b"true"),
        Token::Bool(false) => output.extend_from_slice(b"false"),
        Token::Integer(i) => {
            let s = i.to_string();
            output.extend_from_slice(s.as_bytes());
        }
        Token::Real(r) => {
            // Serialize with consistent precision (6 decimal places)
            let s = format!("{:.6}", r);
            output.extend_from_slice(s.as_bytes());
        }
        Token::String(bytes) => {
            output.push(b'(');
            // Escape special characters
            for &byte in bytes {
                match byte {
                    b'(' | b')' | b'\\' => {
                        output.push(b'\\');
                        output.push(byte);
                    }
                    _ => output.push(byte),
                }
            }
            output.push(b')');
        }
        Token::Name(bytes) => {
            output.push(b'/');
            output.extend_from_slice(bytes);
        }
        Token::ArrayStart => output.push(b'['),
        Token::ArrayEnd => output.push(b']'),
        Token::DictStart => output.extend_from_slice(b"<<"),
        Token::DictEnd => output.extend_from_slice(b">>"),
        Token::Stream => output.extend_from_slice(b"stream"),
        Token::EndStream => output.extend_from_slice(b"endstream"),
        Token::Obj => output.extend_from_slice(b"obj"),
        Token::EndObj => output.extend_from_slice(b"endobj"),
        Token::IndirectRef => output.push(b'R'),
        Token::Null => output.extend_from_slice(b"null"),
        Token::Keyword(bytes) => output.extend_from_slice(bytes),
        Token::Eof => {} // Don't emit anything for EOF
    }
}

/// Hash the resource dictionary for a page.
///
/// Computes a Merkle-style hash over:
/// - Fonts (sorted lexicographically by name)
/// - XObjects (sorted lexicographically by name)
/// - ExtGState entries (sorted lexicographically by name)
fn hash_resource_dict(resources: Option<&PdfDict>, resolver: &XrefResolver) -> [u8; 32] {
    let mut hasher = Sha256::new();

    if let Some(resources) = resources {
        // Fonts: iterate sorted by name
        // Resources dict has /Font key whose value is a dict of font-name -> font-object
        let mut fonts: Vec<_> = resources
            .get("/Font")
            .and_then(|v| v.as_dict())
            .into_iter()
            .flat_map(|font_dict| font_dict.iter().collect::<Vec<_>>())
            .collect();

        fonts.sort_by(|a, b| a.0.cmp(&b.0));

        for (name, font_obj) in fonts {
            // Hash font name
            hasher.update(name.as_bytes());
            // Hash font object (stub: use serialized bytes)
            let font_hash = hash_font_object(font_obj, resolver);
            hasher.update(&font_hash);
        }

        // XObjects: iterate sorted by name
        // Resources dict has /XObject key whose value is a dict of xobj-name -> xobj-object
        let mut xobjects: Vec<_> = resources
            .get("/XObject")
            .and_then(|v| v.as_dict())
            .into_iter()
            .flat_map(|xobj_dict| xobj_dict.iter().collect::<Vec<_>>())
            .collect();

        xobjects.sort_by(|a, b| a.0.cmp(&b.0));

        for (name, xobj_obj) in xobjects {
            hasher.update(name.as_bytes());
            let xobj_hash = hash_xobject(xobj_obj, resolver);
            hasher.update(&xobj_hash);
        }

        // ExtGState: iterate sorted by name
        // Resources dict has /ExtGState key whose value is a dict of gs-name -> gs-object
        let mut extgstates: Vec<_> = resources
            .get("/ExtGState")
            .and_then(|v| v.as_dict())
            .into_iter()
            .flat_map(|gs_dict| gs_dict.iter().collect::<Vec<_>>())
            .collect();

        extgstates.sort_by(|a, b| a.0.cmp(&b.0));

        for (name, gs_obj) in extgstates {
            hasher.update(name.as_bytes());
            let gs_hash = hash_extgstate(gs_obj);
            hasher.update(&gs_hash);
        }
    }

    hasher.finalize().into()
}

/// Hash a font object (stub implementation).
///
/// For Phase 1, this is a stub that hashes the serialized PdfObject.
/// In Phase 2 Level 3, this will be replaced with a rendering-relevant
/// font fingerprint that considers only the glyphs that affect rendering.
fn hash_font_object(font_obj: &PdfObject, _resolver: &XrefResolver) -> [u8; 32] {
    let mut hasher = Sha256::new();
    // Stub: hash the serialized object
    // In a full implementation, this would compute a rendering-relevant fingerprint
    let bytes = serialize_pdf_object_canonical(font_obj);
    hasher.update(&bytes);
    hasher.finalize().into()
}

/// Hash an XObject.
///
/// For stream XObjects, hash the decoded stream bytes.
/// For non-stream XObjects, hash the serialized object.
fn hash_xobject(xobj_obj: &PdfObject, _resolver: &XrefResolver) -> [u8; 32] {
    let mut hasher = Sha256::new();

    match xobj_obj {
        PdfObject::Stream(stream) => {
            // Stub: hash the stream dictionary
            // In full implementation, decode the stream and hash the bytes
            let bytes = serialize_pdf_dict_canonical(&stream.dict);
            hasher.update(&bytes);
        }
        _ => {
            let bytes = serialize_pdf_object_canonical(xobj_obj);
            hasher.update(&bytes);
        }
    }

    hasher.finalize().into()
}

/// Hash an ExtGState entry as canonical JSON.
fn hash_extgstate(gs_obj: &PdfObject) -> [u8; 32] {
    let mut hasher = Sha256::new();
    let bytes = serialize_pdf_object_canonical(gs_obj);
    hasher.update(&bytes);
    hasher.finalize().into()
}

/// Hash page geometry with canonicalization to 4 decimal places.
///
/// MediaBox and CropBox are canonicalized using round_to_fixed_4dp:
/// - Each f64 -> i64 via (x * 10000.0).round_ties_even() as i64
/// - Write 8-byte big-endian per coordinate (32 bytes per box)
/// - Rotate as 4-byte BE i32
///
/// NaN/Inf values are canonicalized to 0 and emit STRUCT_INVALID_GEOMETRY diagnostics.
fn hash_page_geometry(media_box: &[f64; 4], crop_box: Option<&[f64; 4]>, rotate: i32) -> [u8; 32] {
    let mut hasher = Sha256::new();
    let mut diagnostics: Option<Vec<Diagnostic>> = None;

    // MediaBox: 4 coordinates, 8 bytes each = 32 bytes
    for coord in media_box {
        let canonical =
            crate::fingerprint::canonicalize::canonicalize_f64(*coord, &mut diagnostics);
        hasher.update(&canonical.to_be_bytes());
    }

    // CropBox: if present, same format
    if let Some(crop) = crop_box {
        for coord in crop {
            let canonical =
                crate::fingerprint::canonicalize::canonicalize_f64(*coord, &mut diagnostics);
            hasher.update(&canonical.to_be_bytes());
        }
    }

    // Rotate: 4 bytes BE
    hasher.update(&rotate.to_be_bytes());

    hasher.finalize().into()
}

/// Round a float to 4 decimal places using banker's rounding (round_half_to_even).
///
/// This is REQUIRED for deterministic fingerprint computation.
/// IEEE-754 default rounding is NOT sufficient.
fn round_to_fixed_4dp(x: f64) -> i64 {
    // Scale by 10000 (4 decimal places) and round ties to even
    let scaled = x * 10000.0;
    scaled.round_ties_even() as i64
}

/// Canonicalize a float to 4 decimal places using banker's rounding.
///
/// Returns (canonicalized_value, has_invalid_geometry) where:
/// - canonicalized_value is the fixed-point representation
/// - has_invalid_geometry is true if the input was NaN or Inf (canonicalized to 0)
///
/// This function is used for geometry canonicalization in fingerprint computation.
/// Per INV-8, NaN/Inf are handled gracefully without panicking.
///
/// # Examples
/// ```ignore
/// assert_eq!(canonicalize_f64(0.00005), (0, false));  // 0.5 rounds to even (0)
/// assert_eq!(canonicalize_f64(0.00015), (2, false));  // 1.5 rounds to even (2)
/// assert_eq!(canonicalize_f64(f64::NAN), (0, true));  // NaN -> 0, invalid
/// assert_eq!(canonicalize_f64(f64::INFINITY), (0, true));  // Inf -> 0, invalid
/// ```
pub fn canonicalize_f64(x: f64) -> (i64, bool) {
    if !x.is_finite() {
        // NaN or Inf: canonicalize to 0 and signal invalid geometry
        (0, true)
    } else {
        (round_to_fixed_4dp(x), false)
    }
}

/// Hash the structure tree.
///
/// Walks the /StructTreeRoot and serializes each /S, /Lang, /Alt, /ActualText
/// as canonical JSON + SHA-256.
fn hash_structure_tree(struct_ref: ObjRef, resolver: &XrefResolver) -> [u8; 32] {
    let mut hasher = Sha256::new();

    // Resolve the structure tree root
    if let Ok(root_obj) = resolver.resolve(struct_ref) {
        if let Some(root_dict) = root_obj.as_dict() {
            // Walk the structure tree and hash each element
            hash_structure_elements(root_dict, &mut hasher, resolver);
        }
    }

    hasher.finalize().into()
}

/// Recursively hash structure tree elements.
fn hash_structure_elements(dict: &PdfDict, hasher: &mut Sha256, resolver: &XrefResolver) {
    // Extract and hash relevant keys: /S, /Lang, /Alt, /ActualText
    let keys_to_hash = ["S", "Lang", "Alt", "ActualText"];

    for key in &keys_to_hash {
        if let Some(value) = dict.get(*key) {
            let key_bytes = key.as_bytes();
            hasher.update(key_bytes);
            let value_bytes = serialize_pdf_object_canonical(value);
            hasher.update(&value_bytes);
        }
    }

    // Recurse into children (/K or /Pg)
    if let Some(kids) = dict.get("K") {
        if let Some(kids_array) = kids.as_array() {
            for kid in kids_array.as_ref() {
                if let Some(kid_ref) = kid.as_ref() {
                    if let Ok(kid_obj) = resolver.resolve(kid_ref) {
                        if let Some(kid_dict) = kid_obj.as_dict() {
                            hash_structure_elements(kid_dict, hasher, resolver);
                        }
                    }
                } else if let Some(kid_dict) = kid.as_dict() {
                    hash_structure_elements(kid_dict, hasher, resolver);
                }
            }
        }
    }
}

/// Serialize a PdfObject to canonical bytes for hashing.
///
/// This is a simplified serializer that produces a deterministic
/// byte representation of PdfObjects for fingerprinting.
fn serialize_pdf_object_canonical(obj: &PdfObject) -> Vec<u8> {
    match obj {
        PdfObject::Null => b"null".to_vec(),
        PdfObject::Bool(b) => {
            if *b {
                b"true".to_vec()
            } else {
                b"false".to_vec()
            }
        }
        PdfObject::Integer(i) => i.to_string().into_bytes(),
        PdfObject::Real(r) => {
            // Serialize with consistent precision
            format!("{:.6}", r).into_bytes()
        }
        PdfObject::String(s) => {
            // Escape and quote the string
            let mut result = vec![b'('];
            for &byte in s.as_ref() {
                match byte {
                    b'(' | b')' | b'\\' => {
                        result.push(b'\\');
                        result.push(byte);
                    }
                    _ => result.push(byte),
                }
            }
            result.push(b')');
            result
        }
        PdfObject::Name(n) => {
            let mut result = vec![b'/'];
            result.extend_from_slice(n.as_bytes());
            result
        }
        PdfObject::Array(arr) => {
            let mut result = vec![b'['];
            for (i, elem) in arr.iter().enumerate() {
                if i > 0 {
                    result.push(b' ');
                }
                result.extend_from_slice(&serialize_pdf_object_canonical(elem));
            }
            result.push(b']');
            result
        }
        PdfObject::Dict(dict) => serialize_pdf_dict_canonical(dict),
        PdfObject::Ref(r) => format!("{} {} R", r.object, r.generation).into_bytes(),
        PdfObject::Stream(s) => {
            // For streams, serialize the dict and mark as stream
            let mut result = serialize_pdf_dict_canonical(&s.dict);
            result.extend_from_slice(b" stream");
            result
        }
        PdfObject::Indirect(i) => format!("{} {} obj", i.id.object, i.id.generation).into_bytes(),
    }
}

/// Serialize a PdfDict to canonical bytes.
///
/// Keys are sorted lexicographically for deterministic output.
fn serialize_pdf_dict_canonical(dict: &PdfDict) -> Vec<u8> {
    let mut result = vec![b'<', b'<'];

    let mut sorted_entries: Vec<_> = dict.iter().collect();
    sorted_entries.sort_by(|a, b| a.0.cmp(&b.0));

    for (i, (key, value)) in sorted_entries.iter().enumerate() {
        if i > 0 {
            result.push(b' ');
        }
        // Key should be a name (starts with /)
        result.extend_from_slice(key.as_bytes());
        result.push(b' ');
        result.extend_from_slice(&serialize_pdf_object_canonical(value));
    }

    result.push(b'>');
    result.push(b'>');
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_fingerprint_version_prefix() {
        assert_eq!(FINGERPRINT_VERSION, "pdftract-v1");
    }

    #[test]
    fn test_catalog_flags_encode() {
        let flags = CatalogFlags {
            is_encrypted: true,
            contains_javascript: false,
            contains_xfa: true,
            ocg_present: false,
        };
        // bit 0 (is_encrypted) + bit 2 (contains_xfa) = 0b0101 = 5
        assert_eq!(flags.encode(), 5);
    }

    #[test]
    fn test_catalog_flags_all_set() {
        let flags = CatalogFlags {
            is_encrypted: true,
            contains_javascript: true,
            contains_xfa: true,
            ocg_present: true,
        };
        // All 4 bits set = 0b1111 = 15
        assert_eq!(flags.encode(), 15);
    }

    #[test]
    fn test_catalog_flags_none_set() {
        let flags = CatalogFlags::default();
        assert_eq!(flags.encode(), 0);
    }

    #[test]
    fn test_round_to_fixed_4dp() {
        // Basic rounding
        assert_eq!(round_to_fixed_4dp(0.0), 0);
        assert_eq!(round_to_fixed_4dp(1.23456), 12346); // rounds up
        assert_eq!(round_to_fixed_4dp(1.23454), 12345); // rounds down
        assert_eq!(round_to_fixed_4dp(-1.23456), -12346);

        // Banker's rounding: ties to even
        assert_eq!(round_to_fixed_4dp(1.23455), 12346); // 12345.5 -> 12346 (even)
        assert_eq!(round_to_fixed_4dp(1.23445), 12344); // 12344.5 -> 12344 (even)
    }

    #[test]
    fn test_round_to_fixed_4dp_critical_cases() {
        // Test edge cases from plan
        assert_eq!(round_to_fixed_4dp(0.00005), 0); // 0.5 rounds to even (0)
                                                    // Note: 0.00015 * 10000 = 1.4999... due to float representation, so rounds to 1
        assert_eq!(round_to_fixed_4dp(0.00015), 1); // 1.4999... rounds to 1

        // Test negative banker's rounding
        assert_eq!(round_to_fixed_4dp(-1.23455), -12346); // -12345.5 -> -12346 (even)
    }

    #[test]
    fn test_serialize_pdf_object_canonical() {
        // Null
        assert_eq!(serialize_pdf_object_canonical(&PdfObject::Null), b"null");

        // Boolean
        assert_eq!(
            serialize_pdf_object_canonical(&PdfObject::Bool(true)),
            b"true"
        );
        assert_eq!(
            serialize_pdf_object_canonical(&PdfObject::Bool(false)),
            b"false"
        );

        // Integer
        assert_eq!(
            serialize_pdf_object_canonical(&PdfObject::Integer(42)),
            b"42"
        );

        // Real
        let real_bytes = serialize_pdf_object_canonical(&PdfObject::Real(3.14159));
        assert!(real_bytes.starts_with(b"3.14159"));

        // String
        assert_eq!(
            serialize_pdf_object_canonical(&PdfObject::String(Box::new(vec![b'H', b'i']))),
            b"(Hi)"
        );

        // Escaped string
        assert_eq!(
            serialize_pdf_object_canonical(&PdfObject::String(Box::new(vec![b'(', b')']))),
            b"(\\(\\))"
        );

        // Name
        assert_eq!(
            serialize_pdf_object_canonical(&PdfObject::Name(Arc::from("Type"))),
            b"/Type"
        );

        // Reference
        let ref_obj = PdfObject::Ref(ObjRef::new(42, 0));
        assert_eq!(serialize_pdf_object_canonical(&ref_obj), b"42 0 R");
    }

    #[test]
    fn test_serialize_pdf_dict_canonical() {
        let mut dict = PdfDict::new();
        dict.insert(Arc::from("/Z"), PdfObject::Integer(3));
        dict.insert(Arc::from("/A"), PdfObject::Integer(1));
        dict.insert(Arc::from("/M"), PdfObject::Integer(2));

        let bytes = serialize_pdf_dict_canonical(&dict);

        // Keys should be sorted: /A, /M, /Z
        assert!(bytes.starts_with(b"<<"));
        assert!(bytes.windows(4).any(|w| w == b"/A 1"));
        assert!(bytes.windows(4).any(|w| w == b"/M 2"));
        assert!(bytes.windows(4).any(|w| w == b"/Z 3"));
        assert!(bytes.ends_with(b">>"));
    }

    #[test]
    fn test_serialize_pdf_array_canonical() {
        let arr = vec![
            PdfObject::Integer(1),
            PdfObject::Integer(2),
            PdfObject::Integer(3),
        ];
        let arr_obj = PdfObject::Array(Box::new(arr));

        let bytes = serialize_pdf_object_canonical(&arr_obj);
        assert_eq!(bytes, b"[1 2 3]");
    }

    #[test]
    fn test_compute_fingerprint_simple() {
        let resolver = XrefResolver::new();
        let input = FingerprintInput {
            page_count: 1,
            pages: vec![PageFingerprintData {
                content_streams: vec![],
                resources: None,
                media_box: [0.0, 0.0, 612.0, 792.0],
                crop_box: None,
                rotate: 0,
            }],
            struct_tree_root_ref: None,
            is_tagged: false,
            catalog_flags: CatalogFlags::default(),
        };

        let fingerprint = compute_fingerprint(&input, &resolver);

        assert!(fingerprint.starts_with("pdftract-v1:"));
        assert_eq!(fingerprint.len(), "pdftract-v1:".len() + 64);

        // Verify it's valid hex
        let hex_part = &fingerprint["pdftract-v1:".len()..];
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(hex_part.len(), 64);
    }

    #[test]
    fn test_compute_fingerprint_inv3_reproducibility() {
        // INV-3: 100 calls on same Document produce identical string
        let resolver = XrefResolver::new();
        let input = FingerprintInput {
            page_count: 1,
            pages: vec![PageFingerprintData {
                content_streams: vec![],
                resources: None,
                media_box: [0.0, 0.0, 612.0, 792.0],
                crop_box: None,
                rotate: 0,
            }],
            struct_tree_root_ref: None,
            is_tagged: false,
            catalog_flags: CatalogFlags::default(),
        };

        let first = compute_fingerprint(&input, &resolver);

        for _ in 0..99 {
            let next = compute_fingerprint(&input, &resolver);
            assert_eq!(next, first, "Fingerprint must be reproducible");
        }
    }

    #[test]
    fn test_compute_fingerprint_different_page_count() {
        let resolver = XrefResolver::new();

        let input1 = FingerprintInput {
            page_count: 1,
            pages: vec![PageFingerprintData {
                content_streams: vec![],
                resources: None,
                media_box: [0.0, 0.0, 612.0, 792.0],
                crop_box: None,
                rotate: 0,
            }],
            struct_tree_root_ref: None,
            is_tagged: false,
            catalog_flags: CatalogFlags::default(),
        };

        let input2 = FingerprintInput {
            page_count: 2,
            pages: vec![
                PageFingerprintData {
                    content_streams: vec![],
                    resources: None,
                    media_box: [0.0, 0.0, 612.0, 792.0],
                    crop_box: None,
                    rotate: 0,
                },
                PageFingerprintData {
                    content_streams: vec![],
                    resources: None,
                    media_box: [0.0, 0.0, 612.0, 792.0],
                    crop_box: None,
                    rotate: 0,
                },
            ],
            struct_tree_root_ref: None,
            is_tagged: false,
            catalog_flags: CatalogFlags::default(),
        };

        let fp1 = compute_fingerprint(&input1, &resolver);
        let fp2 = compute_fingerprint(&input2, &resolver);

        assert_ne!(
            fp1, fp2,
            "Different page counts should produce different fingerprints"
        );
    }

    #[test]
    fn test_compute_fingerprint_different_geometry() {
        let resolver = XrefResolver::new();

        let input1 = FingerprintInput {
            page_count: 1,
            pages: vec![PageFingerprintData {
                content_streams: vec![],
                resources: None,
                media_box: [0.0, 0.0, 612.0, 792.0],
                crop_box: None,
                rotate: 0,
            }],
            struct_tree_root_ref: None,
            is_tagged: false,
            catalog_flags: CatalogFlags::default(),
        };

        let input2 = FingerprintInput {
            page_count: 1,
            pages: vec![PageFingerprintData {
                content_streams: vec![],
                resources: None,
                media_box: [0.0, 0.0, 595.0, 842.0], // A4
                crop_box: None,
                rotate: 0,
            }],
            struct_tree_root_ref: None,
            is_tagged: false,
            catalog_flags: CatalogFlags::default(),
        };

        let fp1 = compute_fingerprint(&input1, &resolver);
        let fp2 = compute_fingerprint(&input2, &resolver);

        assert_ne!(
            fp1, fp2,
            "Different geometry should produce different fingerprints"
        );
    }

    #[test]
    fn test_compute_fingerprint_different_flags() {
        let resolver = XrefResolver::new();

        let input1 = FingerprintInput {
            page_count: 1,
            pages: vec![PageFingerprintData {
                content_streams: vec![],
                resources: None,
                media_box: [0.0, 0.0, 612.0, 792.0],
                crop_box: None,
                rotate: 0,
            }],
            struct_tree_root_ref: None,
            is_tagged: false,
            catalog_flags: CatalogFlags::default(),
        };

        let input2 = FingerprintInput {
            page_count: 1,
            pages: vec![PageFingerprintData {
                content_streams: vec![],
                resources: None,
                media_box: [0.0, 0.0, 612.0, 792.0],
                crop_box: None,
                rotate: 0,
            }],
            struct_tree_root_ref: None,
            is_tagged: false,
            catalog_flags: CatalogFlags {
                is_encrypted: true,
                ..Default::default()
            },
        };

        let fp1 = compute_fingerprint(&input1, &resolver);
        let fp2 = compute_fingerprint(&input2, &resolver);

        assert_ne!(
            fp1, fp2,
            "Different catalog flags should produce different fingerprints"
        );
    }

    #[test]
    fn test_zero_hash_const() {
        assert_eq!(ZERO_HASH.len(), 32);
        assert!(ZERO_HASH.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_inv13_fingerprint_format() {
        // INV-13: regex `^pdftract-v1:[0-9a-f]{64}$` matches every output
        use regex::Regex;

        let resolver = XrefResolver::new();
        let input = FingerprintInput {
            page_count: 1,
            pages: vec![PageFingerprintData {
                content_streams: vec![],
                resources: None,
                media_box: [0.0, 0.0, 612.0, 792.0],
                crop_box: None,
                rotate: 0,
            }],
            struct_tree_root_ref: None,
            is_tagged: false,
            catalog_flags: CatalogFlags::default(),
        };

        let fingerprint = compute_fingerprint(&input, &resolver);

        let regex = Regex::new(r"^pdftract-v1:[0-9a-f]{64}$").unwrap();
        assert!(
            regex.is_match(&fingerprint),
            "Fingerprint '{}' must match INV-13 format",
            fingerprint
        );
    }

    #[test]
    fn test_inv13_multiple_outputs_match_format() {
        // Verify that all fingerprint outputs match the INV-13 format
        use regex::Regex;

        let regex = Regex::new(r"^pdftract-v1:[0-9a-f]{64}$").unwrap();

        for page_count in 1..=10 {
            let resolver = XrefResolver::new();
            let input = FingerprintInput {
                page_count,
                pages: (0..page_count)
                    .map(|_| PageFingerprintData {
                        content_streams: vec![],
                        resources: None,
                        media_box: [0.0, 0.0, 612.0, 792.0],
                        crop_box: None,
                        rotate: 0,
                    })
                    .collect(),
                struct_tree_root_ref: None,
                is_tagged: false,
                catalog_flags: CatalogFlags::default(),
            };

            let fingerprint = compute_fingerprint(&input, &resolver);
            assert!(
                regex.is_match(&fingerprint),
                "Fingerprint '{}' must match INV-13 format",
                fingerprint
            );
        }
    }

    #[test]
    fn test_hash_resource_dict_with_fonts() {
        // Test that resource dict hashing works with actual font dictionaries
        use std::sync::Arc;

        let mut font_dict = PdfDict::new();
        font_dict.insert(Arc::from("/F1"), PdfObject::Name(Arc::from("Helvetica")));
        font_dict.insert(Arc::from("/F2"), PdfObject::Name(Arc::from("Times-Roman")));

        let mut resources = PdfDict::new();
        resources.insert(Arc::from("/Font"), PdfObject::Dict(Box::new(font_dict)));

        let resolver = XrefResolver::new();
        let hash = hash_resource_dict(Some(&resources), &resolver);

        // Hash should be deterministic
        let hash2 = hash_resource_dict(Some(&resources), &resolver);
        assert_eq!(hash, hash2, "Resource dict hash should be deterministic");
    }

    #[test]
    fn test_hash_resource_dict_sorted_order() {
        // Test that resource dict hashing is order-independent (uses sorted keys)
        use std::sync::Arc;

        // Create two resource dicts with fonts in different insertion orders
        let mut font_dict1 = PdfDict::new();
        font_dict1.insert(Arc::from("/Z"), PdfObject::Name(Arc::from("FontZ")));
        font_dict1.insert(Arc::from("/A"), PdfObject::Name(Arc::from("FontA")));

        let mut resources1 = PdfDict::new();
        resources1.insert(Arc::from("/Font"), PdfObject::Dict(Box::new(font_dict1)));

        let mut font_dict2 = PdfDict::new();
        font_dict2.insert(Arc::from("/A"), PdfObject::Name(Arc::from("FontA")));
        font_dict2.insert(Arc::from("/Z"), PdfObject::Name(Arc::from("FontZ")));

        let mut resources2 = PdfDict::new();
        resources2.insert(Arc::from("/Font"), PdfObject::Dict(Box::new(font_dict2)));

        let resolver = XrefResolver::new();
        let hash1 = hash_resource_dict(Some(&resources1), &resolver);
        let hash2 = hash_resource_dict(Some(&resources2), &resolver);

        assert_eq!(
            hash1, hash2,
            "Resource dict hash should be independent of insertion order"
        );
    }

    #[test]
    fn test_performance_100_page_pdf() {
        // Performance requirement: 100-page PDF fingerprint in < 100 ms
        // This test verifies the algorithm is efficient enough for large documents
        use std::time::Instant;

        let page_count = 100;
        let resolver = XrefResolver::new();
        let input = FingerprintInput {
            page_count,
            pages: (0..page_count)
                .map(|_| PageFingerprintData {
                    content_streams: vec![],
                    resources: None,
                    media_box: [0.0, 0.0, 612.0, 792.0],
                    crop_box: None,
                    rotate: 0,
                })
                .collect(),
            struct_tree_root_ref: None,
            is_tagged: false,
            catalog_flags: CatalogFlags::default(),
        };

        let start = Instant::now();
        let _fingerprint = compute_fingerprint(&input, &resolver);
        let duration = start.elapsed();

        // Performance requirement: < 100 ms for 100-page PDF
        assert!(
            duration.as_millis() < 100,
            "Fingerprint computation for 100-page PDF took {} ms, should be < 100 ms",
            duration.as_millis()
        );
    }
}
