//! BI/ID inline image parser.
//!
//! This module implements the parser for inline images that begin
//! with `BI` and end with `EI`. It parses the header between BI and ID,
//! then scans the raw image data between ID and the whitespace-preceded EI.
//!
//! # Specification
//!
//! Per ISO 32000-1:2008, section 8.9.7 "Inline Images":
//!
//! ```text
//! BI ... header entries ... ID ... image data ... EI
//! ```
//!
//! - `BI` keyword begins the inline image dictionary
//! - Header entries are alternating `/Name Value` pairs
//! - Shorthand keys are allowed (e.g., `/W` for `/Width`, `/H` for `/Height`)
//! - `ID` keyword ends the header and MUST be followed by exactly one whitespace byte
//! - Image data follows until `EI` keyword preceded by whitespace is encountered
//!
//! # Shorthand Key Expansion
//!
//! Per ISO 32000-1 Table 92:
//! - `/W` -> `/Width`
//! - `/H` -> `/Height`
//! - `/BPC` -> `/BitsPerComponent`
//! - `/CS` -> `/ColorSpace`
//! - `/F` -> `/Filter`
//! - `/DP` -> `/DecodeParms`
//! - `/D` -> `/Decode`
//! - `/IM` -> `/ImageMask`
//! - `/I` -> `/Interpolate`
//! - `/OPI` -> `/OPI`

use crate::diagnostics::{DiagCode, Diagnostic as Diag};
use crate::parser::lexer::{Lexer, Token};
use std::fmt;

/// Whitespace bytes that can precede EI per PDF spec section 8.9.7.
///
/// These are: NULL (0x00), HT (0x09), LF (0x0A), FF (0x0C), CR (0x0D), and Space (0x20).
const EI_PRECEDING_WHITESPACE: [u8; 6] = [0x00, 0x09, 0x0A, 0x0C, 0x0D, 0x20];

/// Shorthand key expansion table (ISO 32000-1 Table 92).
///
/// Maps shorthand keys to their full key names.
const SHORTHAND_EXPANSION: &[(&[u8], &[u8])] = &[
    (b"W", b"Width"),
    (b"H", b"Height"),
    (b"BPC", b"BitsPerComponent"),
    (b"CS", b"ColorSpace"),
    (b"F", b"Filter"),
    (b"DP", b"DecodeParms"),
    (b"D", b"Decode"),
    (b"IM", b"ImageMask"),
    (b"I", b"Interpolate"),
    (b"OPI", b"OPI"),
];

/// Expand a shorthand key to its full form.
///
/// Returns the expanded key if the input is a known shorthand, otherwise
/// returns the input unchanged.
fn expand_shorthand_key(key: &[u8]) -> Vec<u8> {
    for &(shorthand, full) in SHORTHAND_EXPANSION {
        if *key == *shorthand {
            return full.to_vec();
        }
    }
    key.to_vec()
}

/// Inline image header parameters.
///
/// Contains the parsed key-value pairs from the BI...ID sequence.
/// All fields are optional; missing fields indicate the parameter
/// was not specified in the header.
#[derive(Debug, Clone, Default)]
pub struct InlineImageHeader {
    /// Width in samples (required for all images)
    pub width: Option<i64>,
    /// Height in samples (required for all images)
    pub height: Option<i64>,
    /// Color space (name or array)
    pub color_space: Option<ColorSpaceValue>,
    /// Bits per component (1, 2, 4, 8, 12, or 16)
    pub bits_per_component: Option<i64>,
    /// Filter (single name or array of names)
    pub filter: Option<FilterValue>,
    /// Decode parameters (single dict or array of dicts)
    pub decode_parms: Option<DecodeParmsValue>,
    /// Decode array (for color value mapping)
    pub decode: Option<Vec<f64>>,
    /// Image mask (boolean)
    pub image_mask: Option<bool>,
    /// Interpolate (boolean)
    pub interpolate: Option<bool>,
    /// OPI version (for OPI-compatible images)
    pub opi: Option<i64>,
}

/// Color space value in inline image header.
///
/// Can be a name (e.g., `/DeviceRGB`) or an array (for `/Indexed`,
/// `/CalRGB`, `/ICCBased` color spaces).
#[derive(Debug, Clone, PartialEq)]
pub enum ColorSpaceValue {
    /// Name object (e.g., `/DeviceGray`, `/DeviceRGB`, `/DeviceCMYK`)
    Name(String),
    /// Array object (e.g., `[/Indexed /DeviceRGB 255 <0000000>]`)
    Array(Vec<ColorSpaceElement>),
}

/// Element in a color space array.
#[derive(Debug, Clone, PartialEq)]
pub enum ColorSpaceElement {
    /// Name element
    Name(String),
    /// Integer element
    Integer(i64),
    /// String (hex string for lookup table)
    String(Vec<u8>),
}

/// Filter value in inline image header.
///
/// Can be a single name or an array of names (for filter chains).
#[derive(Debug, Clone, PartialEq)]
pub enum FilterValue {
    /// Single filter name (e.g., `/ASCIIHexDecode`, `/FlateDecode`)
    Name(String),
    /// Array of filter names (e.g., `[/ASCII85Decode /FlateDecode]`)
    Array(Vec<String>),
}

/// Decode parameters value in inline image header.
///
/// Can be a single dictionary or an array of dictionaries (for filter chains).
#[derive(Debug, Clone, PartialEq)]
pub enum DecodeParmsValue {
    /// Single dictionary (represented as key-value pairs)
    Dict(Vec<(String, DecodeParmValue)>),
    /// Array of dictionaries
    Array(Vec<Vec<(String, DecodeParmValue)>>),
}

/// Value in a decode parameters dictionary.
#[derive(Debug, Clone, PartialEq)]
pub enum DecodeParmValue {
    /// Integer value
    Integer(i64),
    /// Real value
    Real(f64),
    /// Boolean value
    Bool(bool),
    /// Name value
    Name(String),
    /// String value
    String(Vec<u8>),
}

impl InlineImageHeader {
    /// Create a new empty inline image header.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the header has all required fields.
    ///
    /// Per PDF spec, `/Width`, `/Height`, `/ColorSpace`, and `/BitsPerComponent`
    /// are required for all images except image masks.
    pub fn has_required_fields(&self) -> bool {
        let has_dimensions = self.width.is_some() && self.height.is_some();
        let has_color_space = self.color_space.is_some();
        let has_bpc = self.bits_per_component.is_some();

        // Image masks only require width and height
        if self.image_mask == Some(true) {
            return has_dimensions;
        }

        has_dimensions && has_color_space && has_bpc
    }
}

impl fmt::Display for InlineImageHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InlineImageHeader {{ ")?;
        if let Some(w) = self.width {
            write!(f, "width: {}, ", w)?;
        }
        if let Some(h) = self.height {
            write!(f, "height: {}, ", h)?;
        }
        if let Some(ref cs) = self.color_space {
            write!(f, "color_space: {:?}, ", cs)?;
        }
        if let Some(bpc) = self.bits_per_component {
            write!(f, "bits_per_component: {}, ", bpc)?;
        }
        if let Some(ref filter) = self.filter {
            write!(f, "filter: {:?}, ", filter)?;
        }
        write!(f, "}}")
    }
}

/// Parse the BI...ID inline image header.
///
/// This function parses the inline image header that begins with `BI`
/// and ends with `ID`. It consumes alternating key-value pairs, expands
/// shorthand keys per ISO 32000-1 Table 92, and collects them into an
/// `InlineImageHeader` struct.
///
/// # Arguments
///
/// * `lexer` - The lexer positioned after the `BI` keyword
///
/// # Returns
///
/// - `Ok(InlineImageHeader)` - Successfully parsed header
/// - `Err(Vec<Diagnostic>)` - Parsing failed with diagnostics
///
/// # Example
///
/// ```ignore
/// let mut lexer = Lexer::new(b"/W 10 /H 10 /CS /DeviceGray /BPC 8 /F /ASCIIHexDecode ID");
/// let header = parse_inline_image_header(&mut lexer).unwrap();
/// assert_eq!(header.width, Some(10));
/// ```
pub fn parse_inline_image_header(lexer: &mut Lexer) -> Result<InlineImageHeader, Vec<Diag>> {
    let mut header = InlineImageHeader::new();

    // Parse key-value pairs until we encounter ID
    loop {
        // Skip whitespace and comments before key
        // (lexer already does this in next_token)

        let token = match lexer.next_token() {
            Some(t) => t,
            None => {
                // EOF before ID - malformed header (fatal error)
                let mut diagnostics = Vec::new();
                diagnostics.push(Diag::with_static_no_offset(
                    DiagCode::StructUnexpectedEof,
                    "EOF encountered before ID token in inline image header",
                ));
                return Err(diagnostics);
            }
        };

        match token {
            Token::Keyword(ref kw) if kw == b"ID" => {
                // Found ID - check for required whitespace after it
                validate_id_whitespace(lexer);
                break;
            }
            Token::Name(key_bytes) => {
                // Expand shorthand key
                let expanded_key = expand_shorthand_key(&key_bytes);
                let key_str = String::from_utf8_lossy(&expanded_key).to_string();

                // Parse the value
                let value_token = match lexer.next_token() {
                    Some(t) => t,
                    None => {
                        // Missing value - emit diagnostic to lexer and try to recover
                        lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                            DiagCode::StructInvalidDictValue,
                            format!("Missing value after key /{}", key_str),
                        ));
                        // Recover by skipping to next /Key or ID
                        recover_to_next_key(lexer);
                        continue;
                    }
                };

                // Set the header field based on key
                set_header_field(&mut header, &key_str, value_token, lexer);

                // Continue to next key-value pair
            }
            _ => {
                // Unexpected token - emit diagnostic to lexer and try to recover
                lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                    DiagCode::StructInvalidDictKey,
                    format!("Expected name or ID token, got {:?}", token),
                ));
                // Recover by advancing to next /Key or ID
                recover_to_next_key(lexer);
            }
        }
    }

    Ok(header)
}

/// Scan inline image data from ID to whitespace-preceded EI.
///
/// This function extracts the raw image bytes that follow the `ID` keyword
/// and precede the `EI` keyword when it is preceded by a whitespace byte.
///
/// Per PDF spec section 8.9.7, the EI delimiter must be preceded by whitespace
/// to distinguish it from spurious `EI` sequences that may appear in the
/// compressed image data itself.
///
/// # Arguments
///
/// * `lexer` - The lexer positioned immediately after the `ID` keyword
///             (the whitespace after ID has already been consumed)
///
/// # Returns
///
/// * `Ok((Vec<u8>, usize))` - Image data bytes and total bytes consumed
/// * `Err(Vec<Diagnostic>)` - Parsing failed with diagnostics
///
/// # Whitespace Preceding EI
///
/// The following whitespace bytes can precede EI:
/// - 0x00 (NULL)
/// - 0x09 (HT - horizontal tab)
/// - 0x0A (LF - line feed)
/// - 0x0C (FF - form feed)
/// - 0x0D (CR - carriage return)
/// - 0x20 (Space)
///
/// # Example
///
/// ```ignore
/// let mut lexer = Lexer::new(b"ABCD\nEI");
/// let (data, consumed) = scan_inline_image_data(&mut lexer).unwrap();
/// assert_eq!(data, b"ABCD");
/// assert_eq!(consumed, 6); // "ABCD" + "\n" + "EI"
/// ```
pub fn scan_inline_image_data(lexer: &mut Lexer) -> Result<(Vec<u8>, usize), Vec<Diag>> {
    let remaining = lexer.remaining_bytes().to_vec();

    // Empty image (ID EI immediately) - valid
    if remaining.is_empty() {
        lexer.push_diagnostic(Diag::with_static_no_offset(
            DiagCode::InlineImageNoEi,
            "Inline image has no data and no EI terminator (empty image)",
        ));
        return Ok((Vec::new(), 0));
    }

    // Scan byte-by-byte looking for [ws, 0x45, 0x49]
    let mut i = 0;
    let data_len = remaining.len();

    while i < data_len {
        let byte = remaining[i];

        // Check if this byte could be whitespace preceding EI
        if EI_PRECEDING_WHITESPACE.contains(&byte) {
            // Check if we have enough bytes for "EI" (need current byte + 2 more)
            if i + 2 < data_len {
                let next_e = remaining[i + 1];
                let next_i = remaining[i + 2];

                if next_e == 0x45 && next_i == 0x49 {
                    // Found whitespace-preceded EI
                    let image_bytes = remaining[..i].to_vec();
                    let bytes_consumed = i + 3; // data + ws + "EI"

                    // Advance the lexer past the EI
                    lexer.skip_bytes(bytes_consumed as u64);

                    return Ok((image_bytes, bytes_consumed));
                }
            }
        }

        i += 1;
    }

    // No EI found - this is malformed but we should return what we have
    lexer.push_diagnostic(Diag::with_static_no_offset(
        DiagCode::InlineImageNoEi,
        "Inline image data missing EI terminator - consuming to end of stream",
    ));

    // Consume all remaining bytes as image data
    let bytes_consumed = remaining.len();

    // Advance the lexer to the end
    lexer.skip_bytes(bytes_consumed as u64);

    Ok((remaining, bytes_consumed))
}

/// Validate that ID is followed by exactly one whitespace byte.
///
/// Per PDF spec section 8.9.7, the ID keyword must be followed by exactly
/// one whitespace byte (LF, CR, or space). If not, emit a diagnostic.
fn validate_id_whitespace(lexer: &mut Lexer) {
    let remaining = lexer.remaining_bytes();

    // Check if the next byte is a valid whitespace character
    let has_whitespace = remaining
        .first()
        .map_or(false, |&b| matches!(b, b'\n' | b'\r' | b' '));

    if !has_whitespace {
        lexer.push_diagnostic(Diag::with_static_no_offset(
            DiagCode::InlineImageIdWhitespaceMissing,
            "ID token must be followed by exactly one whitespace byte (LF, CR, or space)",
        ));
    }
}

/// Set a header field based on key and value token.
fn set_header_field(
    header: &mut InlineImageHeader,
    key: &str,
    value_token: Token,
    lexer: &mut Lexer,
) {
    match key {
        "Width" => {
            if let Token::Integer(w) = value_token {
                header.width = Some(w);
            } else {
                lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                    DiagCode::StructInvalidType,
                    format!("Expected integer for /Width, got {:?}", value_token),
                ));
            }
        }
        "Height" => {
            if let Token::Integer(h) = value_token {
                header.height = Some(h);
            } else {
                lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                    DiagCode::StructInvalidType,
                    format!("Expected integer for /Height, got {:?}", value_token),
                ));
            }
        }
        "ColorSpace" => {
            if let Some(cs) = parse_color_space_value(value_token, lexer) {
                header.color_space = Some(cs);
            }
        }
        "BitsPerComponent" => {
            if let Token::Integer(bpc) = value_token {
                header.bits_per_component = Some(bpc);
            } else {
                lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                    DiagCode::StructInvalidType,
                    format!(
                        "Expected integer for /BitsPerComponent, got {:?}",
                        value_token
                    ),
                ));
            }
        }
        "Filter" => {
            if let Some(filter) = parse_filter_value(value_token, lexer) {
                header.filter = Some(filter);
            }
        }
        "DecodeParms" => {
            if let Some(decode_parms) = parse_decode_parms_value(value_token, lexer) {
                header.decode_parms = Some(decode_parms);
            }
        }
        "Decode" => {
            if let Some(decode) = parse_decode_array(value_token, lexer) {
                header.decode = Some(decode);
            }
        }
        "ImageMask" => {
            if let Token::Bool(im) = value_token {
                header.image_mask = Some(im);
            } else {
                lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                    DiagCode::StructInvalidType,
                    format!("Expected boolean for /ImageMask, got {:?}", value_token),
                ));
            }
        }
        "Interpolate" => {
            if let Token::Integer(i) = value_token {
                // PDF spec allows boolean or integer (0 or 1)
                header.interpolate = Some(i != 0);
            } else if let Token::Bool(b) = value_token {
                header.interpolate = Some(b);
            } else {
                lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                    DiagCode::StructInvalidType,
                    format!(
                        "Expected boolean or integer for /Interpolate, got {:?}",
                        value_token
                    ),
                ));
            }
        }
        "OPI" => {
            if let Token::Integer(opi) = value_token {
                header.opi = Some(opi);
            } else {
                lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                    DiagCode::StructInvalidType,
                    format!("Expected integer for /OPI, got {:?}", value_token),
                ));
            }
        }
        _ => {
            // Unknown key - emit diagnostic but continue
            lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                DiagCode::StructMissingKey,
                format!("Unknown inline image header key: /{}", key),
            ));
        }
    }
}

/// Parse a color space value from a token.
fn parse_color_space_value(token: Token, lexer: &mut Lexer) -> Option<ColorSpaceValue> {
    match token {
        Token::Name(name_bytes) => {
            let name = String::from_utf8_lossy(&name_bytes).to_string();
            Some(ColorSpaceValue::Name(name))
        }
        Token::ArrayStart => {
            // Parse array elements until ArrayEnd
            let mut elements = Vec::new();
            loop {
                let next_token = match lexer.next_token() {
                    Some(t) => t,
                    None => {
                        lexer.push_diagnostic(Diag::with_static_no_offset(
                            DiagCode::StructUnexpectedEof,
                            "EOF while parsing color space array",
                        ));
                        break;
                    }
                };
                match next_token {
                    Token::ArrayEnd => break,
                    Token::Name(name_bytes) => {
                        let name = String::from_utf8_lossy(&name_bytes).to_string();
                        elements.push(ColorSpaceElement::Name(name));
                    }
                    Token::Integer(i) => {
                        elements.push(ColorSpaceElement::Integer(i));
                    }
                    Token::String(bytes) => {
                        elements.push(ColorSpaceElement::String(bytes));
                    }
                    _ => {
                        lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                            DiagCode::StructInvalidType,
                            format!("Invalid color space array element: {:?}", next_token),
                        ));
                        break;
                    }
                }
            }
            Some(ColorSpaceValue::Array(elements))
        }
        _ => {
            lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                DiagCode::StructInvalidType,
                format!("Expected name or array for /ColorSpace, got {:?}", token),
            ));
            None
        }
    }
}

/// Parse a filter value from a token.
fn parse_filter_value(token: Token, lexer: &mut Lexer) -> Option<FilterValue> {
    match token {
        Token::Name(name_bytes) => {
            let name = String::from_utf8_lossy(&name_bytes).to_string();
            Some(FilterValue::Name(name))
        }
        Token::ArrayStart => {
            // Parse array of names
            let mut names = Vec::new();
            loop {
                let next_token = match lexer.next_token() {
                    Some(t) => t,
                    None => {
                        lexer.push_diagnostic(Diag::with_static_no_offset(
                            DiagCode::StructUnexpectedEof,
                            "EOF while parsing filter array",
                        ));
                        break;
                    }
                };
                match next_token {
                    Token::ArrayEnd => break,
                    Token::Name(name_bytes) => {
                        let name = String::from_utf8_lossy(&name_bytes).to_string();
                        names.push(name);
                    }
                    _ => {
                        lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                            DiagCode::StructInvalidType,
                            format!("Invalid filter array element: {:?}", next_token),
                        ));
                        break;
                    }
                }
            }
            Some(FilterValue::Array(names))
        }
        _ => {
            lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                DiagCode::StructInvalidType,
                format!("Expected name or array for /Filter, got {:?}", token),
            ));
            None
        }
    }
}

/// Parse a decode parameters value from a token.
fn parse_decode_parms_value(token: Token, lexer: &mut Lexer) -> Option<DecodeParmsValue> {
    match token {
        Token::DictStart => {
            // Parse dictionary key-value pairs
            let mut dict = Vec::new();
            loop {
                let next_token = match lexer.next_token() {
                    Some(t) => t,
                    None => {
                        lexer.push_diagnostic(Diag::with_static_no_offset(
                            DiagCode::StructUnexpectedEof,
                            "EOF while parsing decode parms dict",
                        ));
                        break;
                    }
                };
                match next_token {
                    Token::DictEnd => break,
                    Token::Name(key_bytes) => {
                        let key = String::from_utf8_lossy(&key_bytes).to_string();
                        // Parse value (simplified - full implementation would handle all types)
                        // For now, we skip complex nested structures
                        dict.push((key, DecodeParmValue::Integer(0)));
                    }
                    _ => break,
                }
            }
            Some(DecodeParmsValue::Dict(dict))
        }
        Token::ArrayStart => {
            // Parse array of dictionaries
            let mut dicts = Vec::new();
            loop {
                let next_token = match lexer.next_token() {
                    Some(t) => t,
                    None => {
                        lexer.push_diagnostic(Diag::with_static_no_offset(
                            DiagCode::StructUnexpectedEof,
                            "EOF while parsing decode parms array",
                        ));
                        break;
                    }
                };
                match next_token {
                    Token::ArrayEnd => break,
                    Token::DictStart => {
                        let dict = Vec::new();
                        // Parse dictionary (simplified)
                        dicts.push(dict);
                    }
                    _ => break,
                }
            }
            Some(DecodeParmsValue::Array(dicts))
        }
        _ => {
            lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                DiagCode::StructInvalidType,
                format!("Expected dict or array for /DecodeParms, got {:?}", token),
            ));
            None
        }
    }
}

/// Parse a decode array from a token.
fn parse_decode_array(token: Token, lexer: &mut Lexer) -> Option<Vec<f64>> {
    match token {
        Token::ArrayStart => {
            let mut values = Vec::new();
            loop {
                let next_token = match lexer.next_token() {
                    Some(t) => t,
                    None => {
                        lexer.push_diagnostic(Diag::with_static_no_offset(
                            DiagCode::StructUnexpectedEof,
                            "EOF while parsing decode array",
                        ));
                        break;
                    }
                };
                match next_token {
                    Token::ArrayEnd => break,
                    Token::Integer(i) => {
                        values.push(i as f64);
                    }
                    Token::Real(f) => {
                        values.push(f);
                    }
                    _ => {
                        lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                            DiagCode::StructInvalidType,
                            format!("Invalid decode array element: {:?}", next_token),
                        ));
                        break;
                    }
                }
            }
            Some(values)
        }
        _ => {
            lexer.push_diagnostic(Diag::with_dynamic_no_offset(
                DiagCode::StructInvalidType,
                format!("Expected array for /Decode, got {:?}", token),
            ));
            None
        }
    }
}

/// Recover to the next name token or ID keyword.
///
/// This function advances the lexer until it finds a name token (starting
/// with `/`) or the `ID` keyword. It's used for error recovery when a
/// malformed header is encountered.
///
/// The recovery scans byte-by-byte for:
/// - `/` (start of a name token)
/// - `I` followed by `D` (start of the ID keyword)
///
/// This allows the parser to skip past malformed key-value pairs and
/// continue parsing from the next valid key or the ID terminator.
fn recover_to_next_key(lexer: &mut Lexer) {
    let remaining = lexer.remaining_bytes();

    // Scan byte-by-byte for '/' or "ID"
    let mut i = 0;
    while i < remaining.len() {
        let byte = remaining[i];

        if byte == b'/' {
            // Found the start of a name token
            // Skip all bytes before this '/'
            lexer.skip_bytes(i as u64);
            return;
        }

        if byte == b'I' && i + 1 < remaining.len() && remaining[i + 1] == b'D' {
            // Found "ID" - check that it's a token boundary
            // (preceded by whitespace or delimiter, followed by whitespace or delimiter)
            let preceded_by_delim = if i == 0 {
                true // At start of input, so it's a boundary
            } else {
                let prev = remaining[i - 1];
                prev == b' '
                    || prev == b'\t'
                    || prev == b'\n'
                    || prev == b'\r'
                    || prev == b'\x0C'
                    || prev == b'('
                    || prev == b')'
                    || prev == b'<'
                    || prev == b'>'
                    || prev == b'['
                    || prev == b']'
                    || prev == b'{'
                    || prev == b'}'
                    || prev == b'/'
                    || prev == b'%'
            };

            let followed_by_delim = if i + 2 >= remaining.len() {
                true // At end of input, so it's a boundary
            } else {
                let next = remaining[i + 2];
                next == b' '
                    || next == b'\t'
                    || next == b'\n'
                    || next == b'\r'
                    || next == b'\x0C'
                    || next == b'('
                    || next == b')'
                    || next == b'<'
                    || next == b'>'
                    || next == b'['
                    || next == b']'
                    || next == b'{'
                    || next == b'}'
                    || next == b'/'
                    || next == b'%'
            };

            if preceded_by_delim && followed_by_delim {
                // Found a valid "ID" keyword
                lexer.skip_bytes(i as u64);
                return;
            }
        }

        i += 1;
    }

    // No more keys or ID found - skip to end
    lexer.skip_bytes(remaining.len() as u64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shorthand_expansion() {
        assert_eq!(expand_shorthand_key(b"W"), b"Width");
        assert_eq!(expand_shorthand_key(b"H"), b"Height");
        assert_eq!(expand_shorthand_key(b"BPC"), b"BitsPerComponent");
        assert_eq!(expand_shorthand_key(b"CS"), b"ColorSpace");
        assert_eq!(expand_shorthand_key(b"F"), b"Filter");
        assert_eq!(expand_shorthand_key(b"DP"), b"DecodeParms");
        assert_eq!(expand_shorthand_key(b"D"), b"Decode");
        assert_eq!(expand_shorthand_key(b"IM"), b"ImageMask");
        assert_eq!(expand_shorthand_key(b"I"), b"Interpolate");
        assert_eq!(expand_shorthand_key(b"OPI"), b"OPI");

        // Unknown keys are returned unchanged
        assert_eq!(expand_shorthand_key(b"Unknown"), b"Unknown");
    }

    #[test]
    fn test_inline_image_header_new() {
        let header = InlineImageHeader::new();
        assert!(header.width.is_none());
        assert!(header.height.is_none());
        assert!(header.color_space.is_none());
        assert!(header.bits_per_component.is_none());
    }

    #[test]
    fn test_inline_image_header_has_required_fields() {
        let mut header = InlineImageHeader::new();

        // Empty header lacks required fields
        assert!(!header.has_required_fields());

        // Add width and height only (still missing required fields)
        header.width = Some(10);
        header.height = Some(10);
        assert!(!header.has_required_fields());

        // Add color space and BPC
        header.color_space = Some(ColorSpaceValue::Name("DeviceGray".to_string()));
        header.bits_per_component = Some(8);
        assert!(header.has_required_fields());

        // Image mask only requires dimensions
        header.color_space = None;
        header.bits_per_component = None;
        header.image_mask = Some(true);
        assert!(header.has_required_fields());
    }

    #[test]
    fn test_parse_basic_header() {
        let input = b"/W 10 /H 10 /CS /DeviceGray /BPC 8 /F /ASCIIHexDecode ID";
        let mut lexer = Lexer::new(input);

        // Skip to first name (simulating lexer positioned after BI)
        let result = parse_inline_image_header(&mut lexer);

        assert!(result.is_ok());
        let header = result.unwrap();
        assert_eq!(header.width, Some(10));
        assert_eq!(header.height, Some(10));
        assert_eq!(header.bits_per_component, Some(8));
    }

    #[test]
    fn test_parse_header_with_array_filter() {
        let input = b"/W 100 /H 100 /F [/ASCII85Decode /FlateDecode] ID";
        let mut lexer = Lexer::new(input);

        let result = parse_inline_image_header(&mut lexer);

        assert!(result.is_ok());
        let header = result.unwrap();
        assert_eq!(header.width, Some(100));
        assert_eq!(header.height, Some(100));
        assert!(matches!(header.filter, Some(FilterValue::Array(_))));
    }

    #[test]
    fn test_parse_header_with_missing_value() {
        let input = b"/W 10 /H /BPC 8 ID";
        let mut lexer = Lexer::new(input);

        let result = parse_inline_image_header(&mut lexer);

        // Should succeed with diagnostic (not fatal error)
        assert!(result.is_ok());

        // Check that diagnostic was emitted - the value for /H is /BPC (a Name, not an Integer)
        let diags = lexer.take_diagnostics();
        assert!(diags.iter().any(|d| d.code == DiagCode::StructInvalidType));
    }

    #[test]
    fn test_id_whitespace_validation() {
        // ID with LF (valid)
        let input = b"/W 10 ID\n";
        let mut lexer = Lexer::new(input);
        let _ = parse_inline_image_header(&mut lexer);

        // ID at end of input without whitespace (should emit diagnostic)
        let input2 = b"/W 10 ID";
        let mut lexer2 = Lexer::new(input2);
        let result = parse_inline_image_header(&mut lexer2);
        assert!(result.is_ok());

        let diagnostics = lexer2.take_diagnostics();
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagCode::InlineImageIdWhitespaceMissing));
    }

    #[test]
    fn test_scan_inline_image_data_basic() {
        // Image: ABCD<ws>EI
        let input = b"ABCD\nEI";
        let mut lexer = Lexer::new(input);

        let (data, consumed) = scan_inline_image_data(&mut lexer).unwrap();
        assert_eq!(data, b"ABCD");
        assert_eq!(consumed, 7); // "ABCD" (4) + "\n" (1) + "EI" (2)
    }

    #[test]
    fn test_scan_inline_image_data_with_embedded_ei() {
        // Image: ABCDEI<ws>EI
        // The inner "EI" should NOT be a terminator because it's not preceded by ws
        let input = b"ABCDEI\nEI";
        let mut lexer = Lexer::new(input);

        let (data, consumed) = scan_inline_image_data(&mut lexer).unwrap();
        assert_eq!(data, b"ABCDEI");
        assert_eq!(consumed, 9); // "ABCDEI" (6) + "\n" (1) + "EI" (2)
    }

    #[test]
    fn test_scan_inline_image_data_empty() {
        // Empty image: (nothing)EI
        let input = b"\nEI";
        let mut lexer = Lexer::new(input);

        let (data, consumed) = scan_inline_image_data(&mut lexer).unwrap();
        assert_eq!(data, b"");
        assert_eq!(consumed, 3); // "" (0) + "\n" (1) + "EI" (2)
    }

    #[test]
    fn test_scan_inline_image_data_no_ei() {
        // No EI terminator - should emit diagnostic and return all bytes
        let input = b"ABCDEFGH";
        let mut lexer = Lexer::new(input);

        let result = scan_inline_image_data(&mut lexer);
        assert!(result.is_ok());

        let (data, consumed) = result.unwrap();
        assert_eq!(data, b"ABCDEFGH");
        assert_eq!(consumed, 8);

        // Check that diagnostics were emitted
        let diags = lexer.take_diagnostics();
        assert!(diags.iter().any(|d| d.code == DiagCode::InlineImageNoEi));
    }

    #[test]
    fn test_scan_inline_image_data_various_whitespace() {
        // Test each whitespace byte that can precede EI

        // Space (0x20)
        let input = b"ABCD EI";
        let mut lexer = Lexer::new(input);
        let (data, _) = scan_inline_image_data(&mut lexer).unwrap();
        assert_eq!(data, b"ABCD");

        // HT (0x09)
        let input = b"ABCD\tEI";
        let mut lexer = Lexer::new(input);
        let (data, _) = scan_inline_image_data(&mut lexer).unwrap();
        assert_eq!(data, b"ABCD");

        // FF (0x0C)
        let input = b"ABCD\x0CEI";
        let mut lexer = Lexer::new(input);
        let (data, _) = scan_inline_image_data(&mut lexer).unwrap();
        assert_eq!(data, b"ABCD");

        // CR (0x0D)
        let input = b"ABCD\rEI";
        let mut lexer = Lexer::new(input);
        let (data, _) = scan_inline_image_data(&mut lexer).unwrap();
        assert_eq!(data, b"ABCD");

        // LF (0x0A)
        let input = b"ABCD\nEI";
        let mut lexer = Lexer::new(input);
        let (data, _) = scan_inline_image_data(&mut lexer).unwrap();
        assert_eq!(data, b"ABCD");

        // NULL (0x00)
        let input = b"ABCD\x00EI";
        let mut lexer = Lexer::new(input);
        let (data, _) = scan_inline_image_data(&mut lexer).unwrap();
        assert_eq!(data, b"ABCD");
    }

    #[test]
    fn test_scan_inline_image_data_binary_content() {
        // Test with binary content that includes 0x45 and 0x49 bytes
        // but not preceded by whitespace
        let input = b"\x45\x49\x45\x49\nEI"; // "EIEI\nEI"
        let mut lexer = Lexer::new(input);

        let (data, consumed) = scan_inline_image_data(&mut lexer).unwrap();
        assert_eq!(data, b"\x45\x49\x45\x49"); // All "EI" sequences are part of data
        assert_eq!(consumed, 7); // 4 bytes + "\n" (1) + "EI" (2)
    }

    #[test]
    fn test_scan_inline_image_data_lexer_position() {
        // Verify that the lexer position is advanced correctly
        let input = b"ABCD\nEIrest_of_stream";
        let mut lexer = Lexer::new(input);

        let (data, consumed) = scan_inline_image_data(&mut lexer).unwrap();
        assert_eq!(data, b"ABCD");
        assert_eq!(consumed, 7);

        // After scanning, the lexer should be positioned after EI
        let remaining = lexer.remaining_bytes();
        assert_eq!(remaining, b"rest_of_stream");
    }
}
