//! Document outline (bookmark) traversal.
//!
//! This module implements parsing of the PDF document outline hierarchy (bookmarks),
//! including UTF-16BE BOM detection, PDFDocEncoding decoding, and destination resolution.
//!
//! Per PDF 1.7 spec section 12.3.3 "Document Outline":
//! - The outline is a linked list of outline items
//! - Each item has /First (first child) and /Next (next sibling) pointers
//! - /Count indicates open (positive) or closed (negative) state
//! - /Dest or /A specify the destination

use crate::parser::object::{ObjRef, PdfObject};
use crate::parser::pages::PageDict;
use crate::parser::xref::XrefResolver;
use crate::diagnostics::{Diagnostic, DiagCode};
use std::collections::HashSet;

/// Maximum depth of outline nesting to prevent stack overflow.
///
/// Real-world PDFs rarely exceed 5 levels; 16 is very generous.
const MAX_OUTLINE_DEPTH: u8 = 16;

/// Destination anchor types for outline destinations.
///
/// Per PDF 1.7 spec section 12.3.2.2 "Explicit Destinations":
/// - /XYZ: left, top, zoom (null = retain current view)
/// - /Fit: fit page to window
/// - /FitH: fit width, top coordinate
/// - /FitV: left coordinate, fit height
/// - /FitR: fit rectangle (left, bottom, right, top)
/// - /FitB: fit bounding box to window
/// - /FitBH: fit bbox width, top coordinate
/// - /FitBV: left coordinate, fit bbox height
#[derive(Debug, Clone, PartialEq)]
pub enum DestAnchor {
    /// XYZ destination (left, top, zoom)
    /// Any null value means "retain current view"
    Xyz {
        left: Option<f64>,
        top: Option<f64>,
        zoom: Option<f64>,
    },
    /// Fit page to window
    Fit,
    /// Fit horizontally (top coordinate)
    FitH(Option<f64>),
    /// Fit vertically (left coordinate)
    FitV(Option<f64>),
    /// Fit rectangle (left, bottom, right, top)
    FitR(f64, f64, f64, f64),
    /// Fit bounding box to window
    FitB,
    /// Fit bounding box horizontally (top coordinate)
    FitBH(Option<f64>),
    /// Fit bounding box vertically (left coordinate)
    FitBV(Option<f64>),
}

impl DestAnchor {
    /// Parse a destination anchor from a PDF array.
    ///
    /// The array format is: [page_ref, /TypeName, params...]
    /// We skip the first element (page reference) and parse the type.
    fn from_array(arr: &[PdfObject], start_idx: usize) -> Option<Self> {
        if start_idx >= arr.len() {
            return None;
        }

        // Get the destination type name
        let type_name = arr[start_idx].as_name()?;

        match type_name {
            "XYZ" => {
                // /XYZ left top zoom
                let left = arr.get(start_idx + 1).and_then(|o| o.as_real());
                let top = arr.get(start_idx + 2).and_then(|o| o.as_real());
                let zoom = arr.get(start_idx + 3).and_then(|o| o.as_real());
                Some(DestAnchor::Xyz { left, top, zoom })
            }
            "Fit" => Some(DestAnchor::Fit),
            "FitH" => {
                let top = arr.get(start_idx + 1).and_then(|o| o.as_real());
                Some(DestAnchor::FitH(top))
            }
            "FitV" => {
                let left = arr.get(start_idx + 1).and_then(|o| o.as_real());
                Some(DestAnchor::FitV(left))
            }
            "FitR" => {
                let left = arr.get(start_idx + 1).and_then(|o| o.as_real())?;
                let bottom = arr.get(start_idx + 2).and_then(|o| o.as_real())?;
                let right = arr.get(start_idx + 3).and_then(|o| o.as_real())?;
                let top = arr.get(start_idx + 4).and_then(|o| o.as_real())?;
                Some(DestAnchor::FitR(left, bottom, right, top))
            }
            "FitB" => Some(DestAnchor::FitB),
            "FitBH" => {
                let top = arr.get(start_idx + 1).and_then(|o| o.as_real());
                Some(DestAnchor::FitBH(top))
            }
            "FitBV" => {
                let left = arr.get(start_idx + 1).and_then(|o| o.as_real());
                Some(DestAnchor::FitBV(left))
            }
            _ => None,
        }
    }
}

/// A document outline item (bookmark).
///
/// Represents a single node in the outline hierarchy, with support for
/// nested children via the `children` field.
#[derive(Debug, Clone)]
pub struct Outline {
    /// The outline title text (decoded to UTF-8)
    pub title: String,
    /// Number of visible descendants
    /// - Positive: outline is expanded by default
    /// - Negative: outline is collapsed by default
    /// - Zero: no children
    pub count: i32,
    /// Page index of the destination (0-based), if resolved
    pub dest_page: Option<u32>,
    /// Destination anchor within the page
    pub dest_anchor: Option<DestAnchor>,
    /// Nested child outlines
    pub children: Vec<Outline>,
}

impl Outline {
    /// Create a new outline with default values.
    fn new(title: String) -> Self {
        Outline {
            title,
            count: 0,
            dest_page: None,
            dest_anchor: None,
            children: Vec::new(),
        }
    }
}

/// Result type for outline parsing.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// Decode a PDF text string to UTF-8.
///
/// Per PDF 1.7 spec section "Text String Type":
/// - If the string starts with UTF-16BE BOM (0xFE 0xFF), decode as UTF-16BE
/// - Otherwise, decode as PDFDocEncoding (Latin-1 with named character overrides)
///
/// PDFDocEncoding is defined in PDF spec Annex D.2.
/// It's mostly Latin-1 (ISO-8859-1) with 29 character overrides.
fn decode_pdf_string(bytes: &[u8]) -> Result<String> {
    // Check for UTF-16BE BOM
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        return decode_utf16be_bom(&bytes[2..]);
    }

    // Check for UTF-16BE without BOM (heuristic: every other byte is 0x00 for non-ASCII)
    // This is a best-effort heuristic; some producers omit the BOM
    if looks_like_utf16be(bytes) {
        if let Ok(s) = decode_utf16be_raw(bytes) {
            return Ok(s);
        }
    }

    // Fall back to PDFDocEncoding
    decode_pdfdocencoding(bytes)
}

/// Decode UTF-16BE string with BOM (bytes after 0xFE 0xFF).
fn decode_utf16be_bom(bytes: &[u8]) -> Result<String> {
    if bytes.len() % 2 != 0 {
        return Err(vec![
            Diagnostic::with_static_no_offset(
                DiagCode::StructInvalidUtf16,
                "STRUCT_INVALID_UTF16: UTF-16BE string has odd length",
            )
        ]);
    }

    let utf16_chars: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&utf16_chars).map_err(|_| {
        vec![
            Diagnostic::with_static_no_offset(
                DiagCode::StructInvalidUtf16,
                "STRUCT_INVALID_UTF16: Invalid UTF-16BE sequence",
            )
        ]
    })
}

/// Decode raw UTF-16BE (without BOM).
fn decode_utf16be_raw(bytes: &[u8]) -> std::result::Result<String, ()> {
    if bytes.len() % 2 != 0 {
        return Err(());
    }

    let utf16_chars: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&utf16_chars).map_err(|_| ())
}

/// Heuristic check if bytes look like UTF-16BE.
///
/// Returns true if:
/// - Length is even
/// - Most high bytes (first byte of each pair) are 0x00
///
/// This detects UTF-16BE encoded ASCII text, where each ASCII character
/// is stored as [0x00, char_code].
fn looks_like_utf16be(bytes: &[u8]) -> bool {
    if bytes.len() < 2 || bytes.len() % 2 != 0 {
        return false;
    }

    // Count how many high bytes are zero
    let mut zero_high_bytes = 0;
    let total_pairs = bytes.len() / 2;

    for chunk in bytes.chunks_exact(2) {
        if chunk[0] == 0x00 {
            zero_high_bytes += 1;
        }
    }

    // If most high bytes are zero (>= 75%), likely UTF-16BE
    zero_high_bytes >= total_pairs * 3 / 4
}

/// Decode PDFDocEncoded string to UTF-8.
///
/// PDFDocEncoding is defined in PDF spec Annex D.2.
/// It's mostly Latin-1 (ISO-8859-1) with 29 character overrides.
fn decode_pdfdocencoding(bytes: &[u8]) -> Result<String> {
    // PDFDocEncoding overrides from spec Table D.2
    // Key: octal value from spec, Value: Unicode codepoint
    fn pdfdoc_override(byte: u8) -> Option<char> {
        match byte {
            0o010 => Some('\u{0000}'),      // NUL
            0o011 => Some('\u{0001}'),      // SOH
            0o012 => Some('\u{0002}'),      // STX
            0o013 => Some('\u{0003}'),      // ETX
            0o014 => Some('\u{0004}'),      // EOT
            0o015 => Some('\u{0005}'),      // ENQ
            0o016 => Some('\u{0006}'),      // ACK
            0o017 => Some('\u{0007}'),      // BEL
            0o020 => Some('\u{0008}'),      // BS
            0o021 => Some('\u{0009}'),      // HT
            0o022 => Some('\u{000A}'),      // LF
            0o023 => Some('\u{000B}'),      // VT
            0o024 => Some('\u{000C}'),      // FF
            0o025 => Some('\u{000D}'),      // CR
            0o026 => Some('\u{000E}'),      // SO
            0o027 => Some('\u{000F}'),      // SI
            0o030 => Some('\u{0010}'),      // DLE
            0o031 => Some('\u{0011}'),      // DC1
            0o032 => Some('\u{0012}'),      // DC2
            0o033 => Some('\u{0013}'),      // DC3
            0o034 => Some('\u{0014}'),      // DC4
            0o035 => Some('\u{0015}'),      // NAK
            0o036 => Some('\u{0016}'),      // SYN
            0o037 => Some('\u{0017}'),      // ETB
            0o040 => Some('\u{0020}'),      // Space (same as Latin-1)
            0o041 => Some('\u{0021}'),      // !
            0o042 => Some('\u{0022}'),      // "
            0o043 => Some('\u{0023}'),      // #
            0o044 => Some('\u{0024}'),      // $
            0o045 => Some('\u{0025}'),      // %
            0o046 => Some('\u{0026}'),      // &
            0o047 => Some('\u{0027}'),      // '
            0o050 => Some('\u{0028}'),      // (
            0o051 => Some('\u{0029}'),      // )
            0o052 => Some('\u{002A}'),      // *
            0o053 => Some('\u{002B}'),      // +
            0o054 => Some('\u{002C}'),      // ,
            0o055 => Some('\u{002D}'),      // -
            0o056 => Some('\u{002E}'),      // .
            0o057 => Some('\u{002F}'),      // /
            0o060 => Some('\u{0030}'),      // 0
            0o061 => Some('\u{0031}'),      // 1
            0o062 => Some('\u{0032}'),      // 2
            0o063 => Some('\u{0033}'),      // 3
            0o064 => Some('\u{0034}'),      // 4
            0o065 => Some('\u{0035}'),      // 5
            0o066 => Some('\u{0036}'),      // 6
            0o067 => Some('\u{0037}'),      // 7
            0o070 => Some('\u{0038}'),      // 8
            0o071 => Some('\u{0039}'),      // 9
            0o072 => Some('\u{003A}'),      // :
            0o073 => Some('\u{003B}'),      // ;
            0o074 => Some('\u{003C}'),      // <
            0o075 => Some('\u{003D}'),      // =
            0o076 => Some('\u{003E}'),      // >
            0o077 => Some('\u{003F}'),      // ?
            0o100 => Some('\u{0040}'),      // @
            0o101 => Some('\u{0041}'),      // A
            0o102 => Some('\u{0042}'),      // B
            0o103 => Some('\u{0043}'),      // C
            0o104 => Some('\u{0044}'),      // D
            0o105 => Some('\u{0045}'),      // E
            0o106 => Some('\u{0046}'),      // F
            0o107 => Some('\u{0047}'),      // G
            0o110 => Some('\u{0048}'),      // H
            0o111 => Some('\u{0049}'),      // I
            0o112 => Some('\u{004A}'),      // J
            0o113 => Some('\u{004B}'),      // K
            0o114 => Some('\u{004C}'),      // L
            0o115 => Some('\u{004D}'),      // M
            0o116 => Some('\u{004E}'),      // N
            0o117 => Some('\u{004F}'),      // O
            0o120 => Some('\u{0050}'),      // P
            0o121 => Some('\u{0051}'),      // Q
            0o122 => Some('\u{0052}'),      // R
            0o123 => Some('\u{0053}'),      // S
            0o124 => Some('\u{0054}'),      // T
            0o125 => Some('\u{0055}'),      // U
            0o126 => Some('\u{0056}'),      // V
            0o127 => Some('\u{0057}'),      // W
            0o130 => Some('\u{0058}'),      // X
            0o131 => Some('\u{0059}'),      // Y
            0o132 => Some('\u{005A}'),      // Z
            0o133 => Some('\u{005B}'),      // [
            0o134 => Some('\u{005C}'),      // \
            0o135 => Some('\u{005D}'),      // ]
            0o136 => Some('\u{005E}'),      // ^
            0o137 => Some('\u{005F}'),      // _
            0o140 => Some('\u{0060}'),      // `
            0o141 => Some('\u{0061}'),      // a
            0o142 => Some('\u{0062}'),      // b
            0o143 => Some('\u{0063}'),      // c
            0o144 => Some('\u{0064}'),      // d
            0o145 => Some('\u{0065}'),      // e
            0o146 => Some('\u{0066}'),      // f
            0o147 => Some('\u{0067}'),      // g
            0o150 => Some('\u{0068}'),      // h
            0o151 => Some('\u{0069}'),      // i
            0o152 => Some('\u{006A}'),      // j
            0o153 => Some('\u{006B}'),      // k
            0o154 => Some('\u{006C}'),      // l
            0o155 => Some('\u{006D}'),      // m
            0o156 => Some('\u{006E}'),      // n
            0o157 => Some('\u{006F}'),      // o
            0o160 => Some('\u{0070}'),      // p
            0o161 => Some('\u{0071}'),      // q
            0o162 => Some('\u{0072}'),      // r
            0o163 => Some('\u{0073}'),      // s
            0o164 => Some('\u{0074}'),      // t
            0o165 => Some('\u{0075}'),      // u
            0o166 => Some('\u{0076}'),      // v
            0o167 => Some('\u{0077}'),      // w
            0o170 => Some('\u{0078}'),      // x
            0o171 => Some('\u{0079}'),      // y
            0o172 => Some('\u{007A}'),      // z
            0o173 => Some('\u{007B}'),      // {
            0o174 => Some('\u{007C}'),      // |
            0o175 => Some('\u{007D}'),      // }
            0o176 => Some('\u{007E}'),      // ~
            0o200 => Some('\u{2022}'),      // Bullet
            0o201 => Some('\u{2020}'),      // Dagger
            0o202 => Some('\u{2021}'),      // Double Dagger
            0o203 => Some('\u{2026}'),      // Ellipsis
            0o204 => Some('\u{2014}'),      // Em Dash
            0o205 => Some('\u{2013}'),      // En Dash
            0o206 => Some('\u{0192}'),      // Florin
            0o207 => Some('\u{2044}'),      // Fraction
            0o210 => Some('\u{2039}'),      // Single Left Angle Quote
            0o211 => Some('\u{203A}'),      // Single Right Angle Quote
            0o212 => Some('\u{201C}'),      // Double Left Quote
            0o213 => Some('\u{201D}'),      // Double Right Quote
            0o214 => Some('\u{2018}'),      // Single Left Quote
            0o215 => Some('\u{2019}'),      // Single Right Quote
            0o216 => Some('\u{201A}'),      // Single Low-9 Quote
            0o217 => Some('\u{2122}'),      // Trademark
            0o220 => Some('\u{FB01}'),      // fi ligature
            0o221 => Some('\u{FB02}'),      // fl ligature
            0o222 => Some('\u{0141}'),      // L with stroke
            0o223 => Some('\u{0152}'),      // OE ligature
            0o224 => Some('\u{0133}'),      // oe ligature
            0o225 => Some('\u{0178}'),      // Y with diaeresis
            0o226 => Some('\u{00A1}'),      // Inverted exclamation
            0o227 => Some('\u{00BF}'),      // Inverted question mark
            0o230 => Some('\u{00A1}'),      // Inverted exclamation (duplicate in spec)
            0o231 => Some('\u{00BF}'),      // Inverted question mark (duplicate in spec)
            0o232 => Some('\u{00A2}'),      // Cent sign
            0o233 => Some('\u{00A3}'),      // Pound sign
            0o234 => Some('\u{00A5}'),      // Yen sign
            0o235 => Some('\u{20A7}'),      // Peseta sign (changed in PDF 2.0, using original)
            0o236 => Some('\u{0192}'),      // Florin (duplicate)
            0o240 => Some('\u{00E6}'),      // ae ligature
            0o241 => Some('\u{0153}'),      // OE ligature (duplicate)
            0o242 => Some('\u{0178}'),      // Y with diaeresis (duplicate)
            0o243 => Some('\u{00C1}'),      // A with acute
            0o244 => Some('\u{00C2}'),      // A with circumflex
            0o245 => Some('\u{00C4}'),      // A with diaeresis
            0o246 => Some('\u{00C0}'),      // A with grave
            0o247 => Some('\u{00C5}'),      // A with ring
            0o250 => Some('\u{00C7}'),      // C with cedilla
            0o251 => Some('\u{00C9}'),      // E with acute
            0o252 => Some('\u{00C9}'),      // E with acute (duplicate, using correct value)
            0o253 => Some('\u{00CA}'),      // E with circumflex
            0o254 => Some('\u{00CB}'),      // E with diaeresis
            0o255 => Some('\u{00C8}'),      // E with grave
            0o256 => Some('\u{00CD}'),      // I with acute
            0o257 => Some('\u{00CE}'),      // I with circumflex
            0o260 => Some('\u{00CF}'),      // I with diaeresis
            0o261 => Some('\u{00CC}'),      // I with grave
            0o262 => Some('\u{00D1}'),      // N with tilde
            0o263 => Some('\u{00D3}'),      // O with acute
            0o264 => Some('\u{00D4}'),      // O with circumflex
            0o265 => Some('\u{00D6}'),      // O with diaeresis
            0o266 => Some('\u{00D2}'),      // O with grave
            0o267 => Some('\u{00D8}'),      // O with stroke
            0o270 => Some('\u{0152}'),      // OE ligature (duplicate)
            0o271 => Some('\u{00D5}'),      // O with tilde
            0o272 => Some('\u{00D7}'),      // Multiplication
            0o273 => Some('\u{00F7}'),      // Division
            0o274 => Some('\u{0178}'),      // Y with diaeresis (duplicate)
            0o275 => Some('\u{00E1}'),      // a with acute
            0o276 => Some('\u{00E2}'),      // a with circumflex
            0o277 => Some('\u{00E4}'),      // a with diaeresis
            0o300 => Some('\u{00E0}'),      // a with grave
            0o301 => Some('\u{00E5}'),      // a with ring
            0o302 => Some('\u{00E7}'),      // c with cedilla
            0o303 => Some('\u{00E9}'),      // e with acute
            0o304 => Some('\u{00EA}'),      // e with circumflex
            0o305 => Some('\u{00EB}'),      // e with diaeresis
            0o306 => Some('\u{00E8}'),      // e with grave
            0o307 => Some('\u{00ED}'),      // i with acute
            0o310 => Some('\u{00EE}'),      // i with circumflex
            0o311 => Some('\u{00EF}'),      // i with diaeresis
            0o312 => Some('\u{00EC}'),      // i with grave
            0o313 => Some('\u{00F1}'),      // n with tilde
            0o314 => Some('\u{00F3}'),      // o with acute
            0o315 => Some('\u{00F4}'),      // o with circumflex
            0o316 => Some('\u{00F6}'),      // o with diaeresis
            0o317 => Some('\u{00F2}'),      // o with grave
            0o320 => Some('\u{00F8}'),      // o with stroke
            0o321 => Some('\u{0153}'),      // oe ligature
            0o322 => Some('\u{00F5}'),      // o with tilde
            0o323 => Some('\u{00DF}'),      // Sharp s
            0o324 => Some('\u{007B}'),      // { (duplicate)
            0o325 => Some('\u{007D}'),      // } (duplicate)
            0o326 => Some('\u{00A1}'),      // Inverted exclamation (duplicate)
            0o327 => Some('\u{00BF}'),      // Inverted question mark (duplicate)
            0o330 => Some('\u{0161}'),      // s with caron
            0o331 => Some('\u{017D}'),      // Z with caron
            0o332 => Some('\u{00A9}'),      // Copyright
            0o333 => Some('\u{00AE}'),      // Registered
            0o334 => Some('\u{2122}'),      // Trademark (duplicate)
            0o335 => Some('\u{2212}'),      // Minus sign
            0o336 => Some('\u{2012}'),      // Figure dash
            0o337 => Some('\u{0452}'),      // Serbian soft sign
            0o340 => Some('\u{0452}'),      // Serbian soft sign (duplicate)
            0o341 => Some('\u{2013}'),      // En dash (duplicate)
            0o342 => Some('\u{2014}'),      // Em dash (duplicate)
            0o343 => Some('\u{201C}'),      // Double left quote (duplicate)
            0o344 => Some('\u{201D}'),      // Double right quote (duplicate)
            0o345 => Some('\u{2018}'),      // Single left quote (duplicate)
            0o346 => Some('\u{2019}'),      // Single right quote (duplicate)
            0o347 => Some('\u{2022}'),      // Bullet (duplicate)
            0o350 => Some('\u{201A}'),      // Single low-9 quote (duplicate)
            0o351 => Some('\u{2039}'),      // Single left angle quote (duplicate)
            0o352 => Some('\u{203A}'),      // Single right angle quote (duplicate)
            0o353 => Some('\u{2026}'),      // Ellipsis (duplicate)
            0o354 => Some('\u{2020}'),      // Dagger (duplicate)
            0o355 => Some('\u{2021}'),      // Double dagger (duplicate)
            0o356 => Some('\u{20AC}'),      // Euro sign (PDF 1.4+)
            0o357 => Some('\u{2030}'),      // Per mille
            0o360 => Some('\u{0160}'),      // S with caron
            0o361 => Some('\u{017E}'),      // z with caron
            0o362 => Some('\u{0161}'),      // s with caron (duplicate)
            0o363 => Some('\u{017D}'),      // Z with caron (duplicate)
            0o364 => Some('\u{0178}'),      // Y with diaeresis (duplicate)
            0o365 => Some('\u{00A1}'),      // Inverted exclamation (duplicate)
            0o366 => Some('\u{00BF}'),      // Inverted question mark (duplicate)
            0o367 => Some('\u{2212}'),      // Minus sign (duplicate)
            0o370 => Some('\u{0000}'),      // Should be "unused" but using null
            0o371 => Some('\u{0000}'),      // Should be "unused" but using null
            0o372 => Some('\u{0000}'),      // Should be "unused" but using null
            0o373 => Some('\u{0000}'),      // Should be "unused" but using null
            0o374 => Some('\u{0000}'),      // Should be "unused" but using null
            0o375 => Some('\u{0000}'),      // Should be "unused" but using null
            0o376 => Some('\u{0000}'),      // Should be "unused" but using null
            0o377 => Some('\u{0000}'),      // Should be "unused" but using null
            _ => None,
        }
    }

    let result: String = bytes
        .iter()
        .map(|&byte| {
            pdfdoc_override(byte).unwrap_or_else(|| {
                // Default: Latin-1 (ISO-8859-1) interpretation
                (byte as char)
            })
        })
        .collect();

    Ok(result)
}

/// Resolve a destination to a page index and anchor.
///
/// Handles:
/// - /Dest arrays with explicit page reference
/// - /A /GoTo /D (action-based destination)
/// - Named destinations (returns None, emits diagnostic)
fn resolve_destination(
    dest_obj: &PdfObject,
    resolver: &XrefResolver,
    pages: &[PageDict],
    diagnostics: &mut Vec<Diagnostic>,
) -> (Option<u32>, Option<DestAnchor>) {
    // Check if it's an array (explicit destination)
    if let Some(arr) = dest_obj.as_array() {
        if arr.is_empty() {
            return (None, None);
        }

        // First element should be a page reference
        let page_ref = match arr[0].as_ref() {
            Some(ref_) => ref_,
            None => {
                // Named destination - emit diagnostic and return None
                diagnostics.push(Diagnostic::with_static_no_offset(
                    DiagCode::StructUnresolvedDestination,
                    "STRUCT_UNRESOLVED_DESTINATION: Named destination not supported",
                ));
                return (None, None);
            }
        };

        // Look up the page index
        let page_index = pages.iter().position(|p| p.obj_ref == page_ref);

        // Parse the destination anchor (skip first element which is the page ref)
        let dest_anchor = DestAnchor::from_array(arr, 1);

        (page_index.map(|i| i as u32), dest_anchor)
    }
    // Check if it's an action dictionary
    else if let Some(dict) = dest_obj.as_dict() {
        // Check if it's a GoTo action
        if let Some(PdfObject::Name(action_type)) = dict.get("S") {
            if &**action_type == "GoTo" {
                // Recurse on /D (destination array)
                if let Some(dest) = dict.get("D") {
                    return resolve_destination(dest, resolver, pages, diagnostics);
                }
            } else if &**action_type == "URI" {
                // URI action - not a GoTo, so no page destination
                diagnostics.push(Diagnostic::with_static_no_offset(
                    DiagCode::StructNonGotoOutline,
                    "STRUCT_NON_GOTO_OUTLINE: URI action not supported for outline destination",
                ));
                return (None, None);
            }
        }
        (None, None)
    } else if dest_obj.as_name().is_some() || dest_obj.as_string().is_some() {
        // Named destination (name or string) - emit diagnostic and return None
        diagnostics.push(Diagnostic::with_static_no_offset(
            DiagCode::StructUnresolvedDestination,
            "STRUCT_UNRESOLVED_DESTINATION: Named destination not supported",
        ));
        (None, None)
    } else {
        (None, None)
    }
}

/// Parse outline items recursively.
///
/// This is the core traversal function that walks the outline linked list.
/// It maintains cycle detection and depth limits to prevent malformed files
/// from causing stack overflow or infinite loops.
fn parse_outline_recursive(
    node_ref: ObjRef,
    resolver: &XrefResolver,
    pages: &[PageDict],
    visited: &mut HashSet<ObjRef>,
    depth: u8,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Outline> {
    // Cycle detection
    if !visited.insert(node_ref) {
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::StructCircularRef,
            format!("STRUCT_CIRCULAR_REF: Cycle detected at outline node {}", node_ref),
        ));
        return None;
    }

    // Depth limit check
    if depth >= MAX_OUTLINE_DEPTH {
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::StructDepthExceeded,
            format!("STRUCT_DEPTH_EXCEEDED: Outline depth exceeds limit of {}", MAX_OUTLINE_DEPTH),
        ));
        return None;
    }

    // Resolve the outline item dictionary
    let node_obj = match resolver.resolve(node_ref) {
        Ok(obj) => obj,
        Err(e) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Failed to resolve outline node {}: {}", node_ref, e),
            ));
            return None;
        }
    };

    let node_dict = match node_obj.as_dict() {
        Some(d) => d,
        None => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Outline node {} is not a dictionary", node_ref),
            ));
            return None;
        }
    };

    // Extract /Title (required)
    let title = match node_dict.get("Title").and_then(|o| o.as_string()) {
        Some(bytes) => match decode_pdf_string(bytes) {
            Ok(s) => s,
            Err(mut diags) => {
                diagnostics.append(&mut diags);
                String::from("<invalid title>")
            }
        },
        None => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructMissingKey,
                format!("STRUCT_MISSING_KEY: Outline node {} missing /Title", node_ref),
            ));
            String::from("<missing title>")
        }
    };

    let mut outline = Outline::new(title);

    // Extract /Count (optional)
    if let Some(count_val) = node_dict.get("Count").and_then(|o| o.as_int()) {
        outline.count = count_val as i32;
    }

    // Extract /Dest or /A (optional)
    if let Some(dest) = node_dict.get("Dest") {
        let (page_index, dest_anchor) = resolve_destination(dest, resolver, pages, diagnostics);
        outline.dest_page = page_index;
        outline.dest_anchor = dest_anchor;
    } else if let Some(action) = node_dict.get("A") {
        let (page_index, dest_anchor) = resolve_destination(action, resolver, pages, diagnostics);
        outline.dest_page = page_index;
        outline.dest_anchor = dest_anchor;
    }

    // Recurse into children via /First
    if let Some(PdfObject::Ref(first_ref)) = node_dict.get("First") {
        // Walk the sibling list starting at /First
        let mut current_sibling = *first_ref;
        while let Some(child) = parse_outline_recursive(
            current_sibling,
            resolver,
            pages,
            visited,
            depth + 1,
            diagnostics,
        ) {
            outline.children.push(child);

            // Move to /Next sibling
            // Re-resolve to get the /Next reference
            let sibling_obj = match resolver.resolve(current_sibling) {
                Ok(obj) => obj,
                Err(_) => break,
            };

            let sibling_dict = match sibling_obj.as_dict() {
                Some(d) => d,
                None => break,
            };

            match sibling_dict.get("Next").and_then(|o| o.as_ref()) {
                Some(next_ref) => current_sibling = next_ref,
                None => break,
            }
        }
    }

    Some(outline)
}

/// Parse the document outline (bookmarks).
///
/// # Arguments
/// * `resolver` - The xref resolver for resolving indirect references
/// * `outlines_ref` - Optional reference to the /Outlines dictionary
/// * `pages` - Slice of PageDict for resolving destination page indices
///
/// # Returns
/// A vector of top-level outline items, or empty vector if no outlines exist.
///
/// # Behavior
/// - If outlines_ref is None, returns an empty vector (no outlines in document)
/// - Starts traversal at /First of the outlines dictionary
/// - Emits diagnostics for cycles, depth limits, and malformed structures
/// - Never panics; all errors become diagnostics
pub fn parse_outlines(
    resolver: &XrefResolver,
    outlines_ref: Option<ObjRef>,
    pages: &[PageDict],
) -> (Vec<Outline>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut outlines = Vec::new();

    let outlines_root_ref = match outlines_ref {
        Some(ref_) => ref_,
        None => return (outlines, diagnostics), // No outlines in document
    };

    // Resolve the outlines root dictionary
    let root_obj = match resolver.resolve(outlines_root_ref) {
        Ok(obj) => obj,
        Err(e) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Failed to resolve /Outlines root: {}", e),
            ));
            return (outlines, diagnostics);
        }
    };

    let root_dict = match root_obj.as_dict() {
        Some(d) => d,
        None => {
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::StructUnexpectedEof,
                "/Outlines root is not a dictionary",
            ));
            return (outlines, diagnostics);
        }
    };

    // Start traversal at /First (first top-level outline item)
    let mut visited = HashSet::new();
    let mut current_ref = match root_dict.get("First").and_then(|o| o.as_ref()) {
        Some(ref_) => ref_,
        None => return (outlines, diagnostics), // No outlines (empty outline tree)
    };

    // Walk the top-level sibling list
    while let Some(outline) = parse_outline_recursive(
        current_ref,
        resolver,
        pages,
        &mut visited,
        0,
        &mut diagnostics,
    ) {
        outlines.push(outline);

        // Move to /Next sibling
        let current_obj = match resolver.resolve(current_ref) {
            Ok(obj) => obj,
            Err(_) => break,
        };

        let current_dict = match current_obj.as_dict() {
            Some(d) => d,
            None => break,
        };

        match current_dict.get("Next").and_then(|o| o.as_ref()) {
            Some(next_ref) => current_ref = next_ref,
            None => break,
        }
    }

    (outlines, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::intern;
    use crate::parser::resources::ResourceDict;
    use indexmap::IndexMap;
    use std::sync::Arc;

    fn make_test_pages() -> Vec<PageDict> {
        vec![
            PageDict {
                obj_ref: ObjRef::new(10, 0),
                media_box: [0.0, 0.0, 612.0, 792.0],
                crop_box: None,
                bleed_box: None,
                trim_box: None,
                art_box: None,
                rotate: 0,
                resources: Arc::new(ResourceDict::default()),
                contents: Vec::new(),
                annots: Vec::new(),
                actual_text: None,
                lang: None,
                aa: None,
            },
            PageDict {
                obj_ref: ObjRef::new(11, 0),
                media_box: [0.0, 0.0, 612.0, 792.0],
                crop_box: None,
                bleed_box: None,
                trim_box: None,
                art_box: None,
                rotate: 0,
                resources: Arc::new(ResourceDict::default()),
                contents: Vec::new(),
                annots: Vec::new(),
                actual_text: None,
                lang: None,
                aa: None,
            },
            PageDict {
                obj_ref: ObjRef::new(12, 0),
                media_box: [0.0, 0.0, 612.0, 792.0],
                crop_box: None,
                bleed_box: None,
                trim_box: None,
                art_box: None,
                rotate: 0,
                resources: Arc::new(ResourceDict::default()),
                contents: Vec::new(),
                annots: Vec::new(),
                actual_text: None,
                lang: None,
                aa: None,
            },
        ]
    }

    #[test]
    fn test_decode_pdf_string_ascii() {
        let ascii = b"Hello World";
        let result = decode_pdf_string(ascii);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello World");
    }

    #[test]
    fn test_decode_pdf_string_utf16be_bom() {
        // UTF-16BE BOM + "Hi" (0x0048 0x0069)
        let utf16be = vec![0xFE, 0xFF, 0x00, 0x48, 0x00, 0x69];
        let result = decode_pdf_string(&utf16be);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hi");
    }

    #[test]
    fn test_decode_pdf_string_utf16be_bom_odd_length() {
        // Odd length after BOM should emit error
        let utf16be = vec![0xFE, 0xFF, 0x00, 0x48, 0x00];
        let result = decode_pdf_string(&utf16be);
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("STRUCT_INVALID_UTF16")));
    }

    #[test]
    fn test_decode_pdf_string_utf16be_no_bom() {
        // UTF-16BE without BOM: every other byte is 0x00
        let utf16be = vec![0x00, 0x48, 0x00, 0x69, 0x00, 0x20, 0x00, 0x57];
        let result = decode_pdf_string(&utf16be);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hi W");
    }

    #[test]
    fn test_decode_pdfdocencoding_bullet() {
        // Byte 0o200 (0x80) in PDFDocEncoding is bullet (U+2022)
        let pdfdoc = vec![0o200];
        let result = decode_pdfdocencoding(&pdfdoc);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "\u{2022}");
    }

    #[test]
    fn test_decode_pdfdocencoding_em_dash() {
        // Byte 0o204 (0x84) in PDFDocEncoding is em dash (U+2014)
        let pdfdoc = vec![0o204];
        let result = decode_pdfdocencoding(&pdfdoc);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "\u{2014}");
    }

    #[test]
    fn test_decode_pdfdocencoding_fi_ligature() {
        // Byte 0o220 (0x90) in PDFDocEncoding is fi ligature (U+FB01)
        let pdfdoc = vec![0o220];
        let result = decode_pdfdocencoding(&pdfdoc);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "\u{FB01}");
    }

    #[test]
    fn test_dest_anchor_xyz() {
        let mut arr = Vec::new();
        arr.push(PdfObject::Ref(ObjRef::new(10, 0)));
        arr.push(PdfObject::Name(intern("XYZ")));
        arr.push(PdfObject::Real(100.0));
        arr.push(PdfObject::Real(700.0));
        arr.push(PdfObject::Real(1.5));

        let anchor = DestAnchor::from_array(&arr, 1);
        assert_eq!(
            anchor,
            Some(DestAnchor::Xyz {
                left: Some(100.0),
                top: Some(700.0),
                zoom: Some(1.5)
            })
        );
    }

    #[test]
    fn test_dest_anchor_fit() {
        let mut arr = Vec::new();
        arr.push(PdfObject::Ref(ObjRef::new(10, 0)));
        arr.push(PdfObject::Name(intern("Fit")));

        let anchor = DestAnchor::from_array(&arr, 1);
        assert_eq!(anchor, Some(DestAnchor::Fit));
    }

    #[test]
    fn test_dest_anchor_fith() {
        let mut arr = Vec::new();
        arr.push(PdfObject::Ref(ObjRef::new(10, 0)));
        arr.push(PdfObject::Name(intern("FitH")));
        arr.push(PdfObject::Real(500.0));

        let anchor = DestAnchor::from_array(&arr, 1);
        assert_eq!(anchor, Some(DestAnchor::FitH(Some(500.0))));
    }

    #[test]
    fn test_dest_anchor_fitr() {
        let mut arr = Vec::new();
        arr.push(PdfObject::Ref(ObjRef::new(10, 0)));
        arr.push(PdfObject::Name(intern("FitR")));
        arr.push(PdfObject::Real(100.0));
        arr.push(PdfObject::Real(200.0));
        arr.push(PdfObject::Real(300.0));
        arr.push(PdfObject::Real(400.0));

        let anchor = DestAnchor::from_array(&arr, 1);
        assert_eq!(anchor, Some(DestAnchor::FitR(100.0, 200.0, 300.0, 400.0)));
    }

    #[test]
    fn test_dest_anchor_unknown_type() {
        let mut arr = Vec::new();
        arr.push(PdfObject::Ref(ObjRef::new(10, 0)));
        arr.push(PdfObject::Name(intern("Unknown")));

        let anchor = DestAnchor::from_array(&arr, 1);
        assert_eq!(anchor, None);
    }

    #[test]
    fn test_parse_outlines_none() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        let (outlines, diags) = parse_outlines(&resolver, None, &pages);
        assert!(outlines.is_empty());
        assert!(diags.is_empty());
    }

    #[test]
    fn test_parse_outlines_simple() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        // Create a simple outline item
        let mut outline_dict = IndexMap::new();
        outline_dict.insert(intern("Title"), PdfObject::String(Box::new(b"Chapter 1".to_vec())));
        outline_dict.insert(intern("Dest"), {
            let mut dest = Vec::new();
            dest.push(PdfObject::Ref(ObjRef::new(10, 0)));
            dest.push(PdfObject::Name(intern("Fit")));
            PdfObject::Array(Box::new(dest))
        });

        resolver.cache_object(ObjRef::new(100, 0), PdfObject::Dict(Box::new(outline_dict)));

        // Create outlines root with /First
        let mut root_dict = IndexMap::new();
        root_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(100, 0)));
        resolver.cache_object(ObjRef::new(99, 0), PdfObject::Dict(Box::new(root_dict)));

        let (outlines, diags) = parse_outlines(&resolver, Some(ObjRef::new(99, 0)), &pages);
        assert_eq!(outlines.len(), 1);
        assert_eq!(outlines[0].title, "Chapter 1");
        assert_eq!(outlines[0].dest_page, Some(0));
        assert_eq!(outlines[0].dest_anchor, Some(DestAnchor::Fit));
        assert!(diags.is_empty());
    }

    #[test]
    fn test_parse_outlines_with_count() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        // Create an outline item with /Count
        let mut outline_dict = IndexMap::new();
        outline_dict.insert(intern("Title"), PdfObject::String(Box::new(b"Section".to_vec())));
        outline_dict.insert(intern("Count"), PdfObject::Integer(-3)); // Collapsed with 3 descendants
        outline_dict.insert(intern("Dest"), {
            let mut dest = Vec::new();
            dest.push(PdfObject::Ref(ObjRef::new(11, 0)));
            dest.push(PdfObject::Name(intern("Fit")));
            PdfObject::Array(Box::new(dest))
        });

        resolver.cache_object(ObjRef::new(100, 0), PdfObject::Dict(Box::new(outline_dict)));

        // Create outlines root
        let mut root_dict = IndexMap::new();
        root_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(100, 0)));
        resolver.cache_object(ObjRef::new(99, 0), PdfObject::Dict(Box::new(root_dict)));

        let (outlines, diags) = parse_outlines(&resolver, Some(ObjRef::new(99, 0)), &pages);
        assert_eq!(outlines.len(), 1);
        assert_eq!(outlines[0].count, -3);
        assert_eq!(outlines[0].dest_page, Some(1));
    }

    #[test]
    fn test_parse_outlines_nested() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        // Create child outline
        let mut child_dict = IndexMap::new();
        child_dict.insert(intern("Title"), PdfObject::String(Box::new(b"Section 1.1".to_vec())));
        child_dict.insert(intern("Dest"), {
            let mut dest = Vec::new();
            dest.push(PdfObject::Ref(ObjRef::new(12, 0)));
            dest.push(PdfObject::Name(intern("Fit")));
            PdfObject::Array(Box::new(dest))
        });

        resolver.cache_object(ObjRef::new(101, 0), PdfObject::Dict(Box::new(child_dict)));

        // Create parent outline with /First pointing to child
        let mut parent_dict = IndexMap::new();
        parent_dict.insert(intern("Title"), PdfObject::String(Box::new(b"Chapter 1".to_vec())));
        parent_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(101, 0)));
        parent_dict.insert(intern("Count"), PdfObject::Integer(1)); // One child

        resolver.cache_object(ObjRef::new(100, 0), PdfObject::Dict(Box::new(parent_dict)));

        // Create outlines root
        let mut root_dict = IndexMap::new();
        root_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(100, 0)));
        resolver.cache_object(ObjRef::new(99, 0), PdfObject::Dict(Box::new(root_dict)));

        let (outlines, diags) = parse_outlines(&resolver, Some(ObjRef::new(99, 0)), &pages);
        assert_eq!(outlines.len(), 1);
        assert_eq!(outlines[0].title, "Chapter 1");
        assert_eq!(outlines[0].children.len(), 1);
        assert_eq!(outlines[0].children[0].title, "Section 1.1");
        assert_eq!(outlines[0].children[0].dest_page, Some(2));
    }

    #[test]
    fn test_parse_outlines_three_level_hierarchy() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        // Level 3: Grandchild
        let mut grandchild_dict = IndexMap::new();
        grandchild_dict.insert(intern("Title"), PdfObject::String(Box::new(b"Section 1.1.1".to_vec())));
        grandchild_dict.insert(intern("Dest"), {
            let mut dest = Vec::new();
            dest.push(PdfObject::Ref(ObjRef::new(10, 0)));
            dest.push(PdfObject::Name(intern("Fit")));
            PdfObject::Array(Box::new(dest))
        });

        resolver.cache_object(ObjRef::new(102, 0), PdfObject::Dict(Box::new(grandchild_dict)));

        // Level 2: Child with /First pointing to grandchild
        let mut child_dict = IndexMap::new();
        child_dict.insert(intern("Title"), PdfObject::String(Box::new(b"Section 1.1".to_vec())));
        child_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(102, 0)));
        child_dict.insert(intern("Count"), PdfObject::Integer(1));

        resolver.cache_object(ObjRef::new(101, 0), PdfObject::Dict(Box::new(child_dict)));

        // Level 1: Parent with /First pointing to child
        let mut parent_dict = IndexMap::new();
        parent_dict.insert(intern("Title"), PdfObject::String(Box::new(b"Chapter 1".to_vec())));
        parent_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(101, 0)));
        parent_dict.insert(intern("Count"), PdfObject::Integer(2));

        resolver.cache_object(ObjRef::new(100, 0), PdfObject::Dict(Box::new(parent_dict)));

        // Create outlines root
        let mut root_dict = IndexMap::new();
        root_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(100, 0)));
        resolver.cache_object(ObjRef::new(99, 0), PdfObject::Dict(Box::new(root_dict)));

        let (outlines, diags) = parse_outlines(&resolver, Some(ObjRef::new(99, 0)), &pages);
        assert_eq!(outlines.len(), 1);
        assert_eq!(outlines[0].title, "Chapter 1");
        assert_eq!(outlines[0].children.len(), 1);
        assert_eq!(outlines[0].children[0].title, "Section 1.1");
        assert_eq!(outlines[0].children[0].children.len(), 1);
        assert_eq!(outlines[0].children[0].children[0].title, "Section 1.1.1");
        assert_eq!(outlines[0].children[0].children[0].dest_page, Some(0));
    }

    #[test]
    fn test_parse_outlines_siblings() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        // Create second sibling
        let mut sibling2_dict = IndexMap::new();
        sibling2_dict.insert(intern("Title"), PdfObject::String(Box::new(b"Chapter 2".to_vec())));
        sibling2_dict.insert(intern("Dest"), {
            let mut dest = Vec::new();
            dest.push(PdfObject::Ref(ObjRef::new(11, 0)));
            dest.push(PdfObject::Name(intern("Fit")));
            PdfObject::Array(Box::new(dest))
        });

        resolver.cache_object(ObjRef::new(101, 0), PdfObject::Dict(Box::new(sibling2_dict)));

        // Create first sibling with /Next pointing to second
        let mut sibling1_dict = IndexMap::new();
        sibling1_dict.insert(intern("Title"), PdfObject::String(Box::new(b"Chapter 1".to_vec())));
        sibling1_dict.insert(intern("Next"), PdfObject::Ref(ObjRef::new(101, 0)));
        sibling1_dict.insert(intern("Dest"), {
            let mut dest = Vec::new();
            dest.push(PdfObject::Ref(ObjRef::new(10, 0)));
            dest.push(PdfObject::Name(intern("Fit")));
            PdfObject::Array(Box::new(dest))
        });

        resolver.cache_object(ObjRef::new(100, 0), PdfObject::Dict(Box::new(sibling1_dict)));

        // Create outlines root
        let mut root_dict = IndexMap::new();
        root_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(100, 0)));
        resolver.cache_object(ObjRef::new(99, 0), PdfObject::Dict(Box::new(root_dict)));

        let (outlines, diags) = parse_outlines(&resolver, Some(ObjRef::new(99, 0)), &pages);
        assert_eq!(outlines.len(), 2);
        assert_eq!(outlines[0].title, "Chapter 1");
        assert_eq!(outlines[1].title, "Chapter 2");
        assert_eq!(outlines[0].dest_page, Some(0));
        assert_eq!(outlines[1].dest_page, Some(1));
    }

    #[test]
    fn test_parse_outlines_cycle_detection() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        // Create an outline that forms a cycle: 100 -> 101 -> 100
        let mut outline1_dict = IndexMap::new();
        outline1_dict.insert(intern("Title"), PdfObject::String(Box::new(b"Outline 1".to_vec())));
        outline1_dict.insert(intern("Next"), PdfObject::Ref(ObjRef::new(101, 0)));

        resolver.cache_object(ObjRef::new(100, 0), PdfObject::Dict(Box::new(outline1_dict)));

        let mut outline2_dict = IndexMap::new();
        outline2_dict.insert(intern("Title"), PdfObject::String(Box::new(b"Outline 2".to_vec())));
        outline2_dict.insert(intern("Next"), PdfObject::Ref(ObjRef::new(100, 0))); // Cycle back

        resolver.cache_object(ObjRef::new(101, 0), PdfObject::Dict(Box::new(outline2_dict)));

        // Create outlines root
        let mut root_dict = IndexMap::new();
        root_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(100, 0)));
        resolver.cache_object(ObjRef::new(99, 0), PdfObject::Dict(Box::new(root_dict)));

        let (outlines, diags) = parse_outlines(&resolver, Some(ObjRef::new(99, 0)), &pages);
        // Should get both outlines before detecting the cycle
        assert_eq!(outlines.len(), 2);
        // Should have a cycle diagnostic
        assert!(diags.iter().any(|d| d.message.contains("STRUCT_CIRCULAR_REF")));
    }

    #[test]
    fn test_parse_outlines_missing_title() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        // Create an outline without /Title
        let mut outline_dict = IndexMap::new();
        // No /Title key
        outline_dict.insert(intern("Dest"), {
            let mut dest = Vec::new();
            dest.push(PdfObject::Ref(ObjRef::new(10, 0)));
            dest.push(PdfObject::Name(intern("Fit")));
            PdfObject::Array(Box::new(dest))
        });

        resolver.cache_object(ObjRef::new(100, 0), PdfObject::Dict(Box::new(outline_dict)));

        // Create outlines root
        let mut root_dict = IndexMap::new();
        root_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(100, 0)));
        resolver.cache_object(ObjRef::new(99, 0), PdfObject::Dict(Box::new(root_dict)));

        let (outlines, diags) = parse_outlines(&resolver, Some(ObjRef::new(99, 0)), &pages);
        assert_eq!(outlines.len(), 1);
        assert_eq!(outlines[0].title, "<missing title>");
        assert!(diags.iter().any(|d| d.message.contains("STRUCT_MISSING_KEY")));
    }

    #[test]
    fn test_parse_outlines_goto_action() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        // Create an outline with /A /GoTo action
        let mut goto_dest = Vec::new();
        goto_dest.push(PdfObject::Ref(ObjRef::new(12, 0)));
        goto_dest.push(PdfObject::Name(intern("XYZ")));
        goto_dest.push(PdfObject::Null); // left = null (retain current)
        goto_dest.push(PdfObject::Real(500.0));
        goto_dest.push(PdfObject::Null); // zoom = null

        let mut action_dict = IndexMap::new();
        action_dict.insert(intern("S"), PdfObject::Name(intern("GoTo")));
        action_dict.insert(intern("D"), PdfObject::Array(Box::new(goto_dest)));

        let mut outline_dict = IndexMap::new();
        outline_dict.insert(intern("Title"), PdfObject::String(Box::new(b"GoTo Test".to_vec())));
        outline_dict.insert(intern("A"), PdfObject::Dict(Box::new(action_dict)));

        resolver.cache_object(ObjRef::new(100, 0), PdfObject::Dict(Box::new(outline_dict)));

        // Create outlines root
        let mut root_dict = IndexMap::new();
        root_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(100, 0)));
        resolver.cache_object(ObjRef::new(99, 0), PdfObject::Dict(Box::new(root_dict)));

        let (outlines, diags) = parse_outlines(&resolver, Some(ObjRef::new(99, 0)), &pages);
        assert_eq!(outlines.len(), 1);
        assert_eq!(outlines[0].title, "GoTo Test");
        assert_eq!(outlines[0].dest_page, Some(2));
        assert_eq!(
            outlines[0].dest_anchor,
            Some(DestAnchor::Xyz {
                left: None,
                top: Some(500.0),
                zoom: None
            })
        );
    }

    #[test]
    fn test_parse_outlines_uri_action() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        // Create an outline with /A /URI action
        let mut action_dict = IndexMap::new();
        action_dict.insert(intern("S"), PdfObject::Name(intern("URI")));
        action_dict.insert(intern("URI"), PdfObject::String(Box::new(b"https://example.com".to_vec())));

        let mut outline_dict = IndexMap::new();
        outline_dict.insert(intern("Title"), PdfObject::String(Box::new(b"External Link".to_vec())));
        outline_dict.insert(intern("A"), PdfObject::Dict(Box::new(action_dict)));

        resolver.cache_object(ObjRef::new(100, 0), PdfObject::Dict(Box::new(outline_dict)));

        // Create outlines root
        let mut root_dict = IndexMap::new();
        root_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(100, 0)));
        resolver.cache_object(ObjRef::new(99, 0), PdfObject::Dict(Box::new(root_dict)));

        let (outlines, diags) = parse_outlines(&resolver, Some(ObjRef::new(99, 0)), &pages);
        assert_eq!(outlines.len(), 1);
        assert_eq!(outlines[0].title, "External Link");
        assert_eq!(outlines[0].dest_page, None);
        assert!(diags.iter().any(|d| d.message.contains("STRUCT_NON_GOTO_OUTLINE")));
    }

    #[test]
    fn test_parse_outlines_named_destination() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        // Create an outline with a named destination (string instead of page ref)
        let mut outline_dict = IndexMap::new();
        outline_dict.insert(intern("Title"), PdfObject::String(Box::new(b"Named Dest".to_vec())));
        outline_dict.insert(intern("Dest"), PdfObject::Name(intern("Chapter1")));

        resolver.cache_object(ObjRef::new(100, 0), PdfObject::Dict(Box::new(outline_dict)));

        // Create outlines root
        let mut root_dict = IndexMap::new();
        root_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(100, 0)));
        resolver.cache_object(ObjRef::new(99, 0), PdfObject::Dict(Box::new(root_dict)));

        let (outlines, diags) = parse_outlines(&resolver, Some(ObjRef::new(99, 0)), &pages);
        assert_eq!(outlines.len(), 1);
        assert_eq!(outlines[0].dest_page, None);
        assert!(diags.iter().any(|d| d.message.contains("STRUCT_UNRESOLVED_DESTINATION")));
    }

    #[test]
    fn test_looks_like_utf16be() {
        // ASCII should not be detected as UTF-16BE
        assert!(!looks_like_utf16be(b"Hello"));

        // UTF-16BE with zero high bytes should be detected
        assert!(looks_like_utf16be(&[0x00, 0x48, 0x00, 0x69]));

        // Odd length should not be detected
        assert!(!looks_like_utf16be(&[0x00, 0x48, 0x00]));

        // All ASCII (< 0x80) should not be detected
        assert!(!looks_like_utf16be(&[0x41, 0x42, 0x43]));
    }

    #[test]
    fn test_empty_outlines() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        // Create outlines root without /First
        let mut root_dict = IndexMap::new();
        // No /First key
        resolver.cache_object(ObjRef::new(99, 0), PdfObject::Dict(Box::new(root_dict)));

        let (outlines, diags) = parse_outlines(&resolver, Some(ObjRef::new(99, 0)), &pages);
        assert!(outlines.is_empty());
        assert!(diags.is_empty());
    }

    #[test]
    fn test_invalid_outlines_root() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        // Outlines root is not a dictionary
        resolver.cache_object(ObjRef::new(99, 0), PdfObject::Integer(42));

        let (outlines, diags) = parse_outlines(&resolver, Some(ObjRef::new(99, 0)), &pages);
        assert!(outlines.is_empty());
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.message.contains("not a dictionary")));
    }

    #[test]
    fn test_outline_with_xyz_null_values() {
        let resolver = XrefResolver::new();
        let pages = make_test_pages();

        // Create an outline with /XYZ destination where left/top/zoom are null
        let mut outline_dict = IndexMap::new();
        outline_dict.insert(intern("Title"), PdfObject::String(Box::new(b"Null Values".to_vec())));
        outline_dict.insert(intern("Dest"), {
            let mut dest = Vec::new();
            dest.push(PdfObject::Ref(ObjRef::new(10, 0)));
            dest.push(PdfObject::Name(intern("XYZ")));
            dest.push(PdfObject::Null); // left = null
            dest.push(PdfObject::Null); // top = null
            dest.push(PdfObject::Null); // zoom = null
            PdfObject::Array(Box::new(dest))
        });

        resolver.cache_object(ObjRef::new(100, 0), PdfObject::Dict(Box::new(outline_dict)));

        // Create outlines root
        let mut root_dict = IndexMap::new();
        root_dict.insert(intern("First"), PdfObject::Ref(ObjRef::new(100, 0)));
        resolver.cache_object(ObjRef::new(99, 0), PdfObject::Dict(Box::new(root_dict)));

        let (outlines, diags) = parse_outlines(&resolver, Some(ObjRef::new(99, 0)), &pages);
        assert_eq!(outlines.len(), 1);
        assert_eq!(
            outlines[0].dest_anchor,
            Some(DestAnchor::Xyz {
                left: None,
                top: None,
                zoom: None
            })
        );
    }
}

/// Property tests for outline parsing fuzzing.
///
/// Per acceptance criteria: "proptest: random outline tree shapes never panic"
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Test that decode_pdf_string never panics on arbitrary input (INV-8).
        #[test]
        fn fuzz_decode_pdf_string_no_panics(bytes in prop::collection::vec(any::<u8>(), 0..1000)) {
            // This should never panic - should always return Ok or Err with diagnostics
            let _ = decode_pdf_string(&bytes);
        }

        /// Test that decode_pdfdocencoding never panics on arbitrary input.
        #[test]
        fn fuzz_decode_pdfdocencoding_no_panics(bytes in prop::collection::vec(any::<u8>(), 0..256)) {
            // This should never panic
            let _ = decode_pdfdocencoding(&bytes);
        }

        /// Test that DestAnchor::from_array never panics on arbitrary input.
        #[test]
        fn fuzz_dest_anchor_from_array_no_panics(
            arr in prop::collection::vec(
                prop::strategy::Just(PdfObject::Null),
                0..20
            )
        ) {
            // This should never panic
            let _ = DestAnchor::from_array(&arr, 0);
            let _ = DestAnchor::from_array(&arr, 5);
        }
    }
}
