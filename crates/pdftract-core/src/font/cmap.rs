//! ToUnicode CMap parser (Level 1).
//!
//! This module implements parsing of the `/ToUnicode` stream from PDF fonts
//! as a PostScript CMap program. It extracts the character code to Unicode
//! mapping used for accurate text extraction.
//!
//! # CMap syntax support
//!
//! - `beginbfchar` / `endbfchar`: Single-character mappings
//! - `beginbfrange` / `endbfrange`: Range mappings (contiguous and explicit array)
//! - `usecmap`: Inheritance from named CMaps (stub - emits diagnostic)
//! - Comments: `%` to end of line (stripped by lexer)
//!
//! # Mapping format
//!
//! Source codes are stored as variable-length byte sequences (1-4 bytes).
//! Destinations are stored as UTF-32 codepoint slices, supporting multi-codepoint
//! mappings like ligature expansion (`fi` → U+0066 U+0069).

use std::collections::HashMap;

use crate::diagnostics::{Diagnostic, DiagCode};
use crate::parser::lexer::Lexer;
use crate::parser::lexer::Token;

/// Result type for CMap operations.
pub type CMapResult<T> = Result<T, CMapError>;

/// Errors that can occur during CMap parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CMapError {
    /// Unexpected token in CMap stream.
    UnexpectedToken(String),
    /// Invalid hex string format.
    InvalidHexString(String),
    /// Invalid range (lo > hi).
    InvalidRange,
    /// Array length mismatch in bfrange.
    ArrayLengthMismatch,
    /// Missing expected keyword (e.g., endbfchar).
    MissingKeyword(String),
    /// Empty CMap (no mappings).
    EmptyCMap,
}

impl std::fmt::Display for CMapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CMapError::UnexpectedToken(msg) => write!(f, "unexpected token: {}", msg),
            CMapError::InvalidHexString(msg) => write!(f, "invalid hex string: {}", msg),
            CMapError::InvalidRange => write!(f, "invalid range: lo > hi"),
            CMapError::ArrayLengthMismatch => write!(f, "bfrange array length does not match range"),
            CMapError::MissingKeyword(kw) => write!(f, "missing expected keyword: {}", kw),
            CMapError::EmptyCMap => write!(f, "CMap contains no mappings"),
        }
    }
}

impl std::error::Error for CMapError {}

/// A ToUnicode CMap mapping.
///
/// Maps source byte sequences to Unicode codepoint slices.
#[derive(Debug, Clone)]
pub struct ToUnicodeMap {
    /// Mapping from source byte sequence to destination Unicode codepoints.
    /// Uses Vec<u8> as key (source bytes) and Vec<char> as value (destination chars).
    mappings: HashMap<Vec<u8>, Vec<char>>,
}

impl ToUnicodeMap {
    /// Create a new empty ToUnicode map.
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    /// Add a single mapping from source bytes to destination chars.
    pub fn add_mapping(&mut self, src: Vec<u8>, dst: Vec<char>) {
        self.mappings.insert(src, dst);
    }

    /// Look up a source byte sequence and return the mapped Unicode characters.
    ///
    /// Returns None if the source sequence is not in the map.
    pub fn lookup(&self, src: &[u8]) -> Option<&[char]> {
        self.mappings.get(src).map(|v| v.as_slice())
    }

    /// Check if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    /// Get the number of mappings in the map.
    pub fn len(&self) -> usize {
        self.mappings.len()
    }
}

impl Default for ToUnicodeMap {
    fn default() -> Self {
        Self::new()
    }
}

/// ToUnicode CMap parser.
///
/// Parses a PostScript CMap program from a ToUnicode stream and extracts
/// character code to Unicode mappings.
pub struct CMapParser<'a> {
    lexer: Lexer<'a>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> CMapParser<'a> {
    /// Create a new CMap parser for the given input bytes.
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::new(input),
            diagnostics: Vec::new(),
        }
    }

    /// Parse the CMap and return the ToUnicode map.
    ///
    /// This consumes the parser and returns the populated map along with
    /// any diagnostics generated during parsing.
    pub fn parse(mut self) -> (ToUnicodeMap, Vec<Diagnostic>) {
        let mut map = ToUnicodeMap::new();

        while let Some(token) = self.lexer.next_token() {
            match token {
                Token::Eof => break,
                Token::Keyword(ref kw) => {
                    match kw.as_slice() {
                        b"beginbfchar" => {
                            if let Err(e) = self.parse_beginbfchar(&mut map) {
                                self.emit_error(&e);
                                // Attempt recovery: skip to endbfchar
                                self.skip_to_keyword(b"endbfchar");
                            }
                        }
                        b"beginbfrange" => {
                            if let Err(e) = self.parse_beginbfrange(&mut map) {
                                self.emit_error(&e);
                                // Attempt recovery: skip to endbfrange
                                self.skip_to_keyword(b"endbfrange");
                            }
                        }
                        b"usecmap" => {
                            self.handle_usecmap();
                        }
                        b"endbfchar" | b"endbfrange" => {
                            // These should have been consumed by their respective parsers
                            // If we see them here, it indicates unbalanced blocks
                            self.diagnostics.push(Diagnostic::with_static(
                                DiagCode::FontInvalidCmap,
                                self.lexer.position(),
                                "Unbalanced CMap block",
                            ));
                        }
                        _ => {
                            // Unknown keyword - skip it
                        }
                    }
                }
                _ => {
                    // Unexpected token - skip it
                }
            }
        }

        // Take diagnostics from lexer as well
        self.diagnostics.extend(self.lexer.take_diagnostics());

        (map, self.diagnostics)
    }

    /// Parse a beginbfchar...endbfchar block.
    ///
    /// Format: beginbfchar <count> <src1> <dst1> <src2> <dst2> ... endbfchar
    fn parse_beginbfchar(&mut self, map: &mut ToUnicodeMap) -> Result<(), CMapError> {
        // Read count
        let count = self.expect_integer()?;
        if count < 0 {
            return Err(CMapError::UnexpectedToken(
                "negative bfchar count".to_string(),
            ));
        }
        let count = count as usize;

        // Read count pairs of <src> <dst>
        for _ in 0..count {
            // Source hex string
            let src = self.expect_hex_string()?;

            // Destination hex string (UTF-16BE)
            let dst_hex = self.expect_hex_string()?;
            let dst = self.decode_utf16be(&dst_hex)?;

            map.add_mapping(src, dst);
        }

        // Expect endbfchar
        self.expect_keyword(b"endbfchar")?;

        Ok(())
    }

    /// Parse a beginbfrange...endbfrange block.
    ///
    /// Two forms:
    /// - beginbfrange <count> <lo> <hi> <dst> ... endbfrange (contiguous)
    /// - beginbfrange <count> <lo> <hi> [<d0> <d1> ...] ... endbfrange (explicit array)
    fn parse_beginbfrange(&mut self, map: &mut ToUnicodeMap) -> Result<(), CMapError> {
        // Read count
        let count = self.expect_integer()?;
        if count < 0 {
            return Err(CMapError::UnexpectedToken(
                "negative bfrange count".to_string(),
            ));
        }
        let count = count as usize;

        for _ in 0..count {
            // Read lo and hi
            let lo = self.expect_hex_string()?;
            let hi = self.expect_hex_string()?;

            // Check if lo <= hi (as byte sequences)
            if lo > hi {
                return Err(CMapError::InvalidRange);
            }

            // Peek at next token to determine form
            let next_token = self.lexer.peek_token().cloned();

            if let Some(Token::ArrayStart) = next_token {
                // Explicit array form: <lo> <hi> [<d0> <d1> ...]
                self.lexer.next_token(); // consume [

                let mut dst_strings = Vec::new();
                loop {
                    match self.lexer.next_token() {
                        Some(Token::String(bytes)) => {
                            let decoded = self.decode_utf16be(&bytes)?;
                            dst_strings.push(decoded);
                        }
                        Some(Token::ArrayEnd) => break,
                        Some(other) => {
                            return Err(CMapError::UnexpectedToken(format!(
                                "expected hex string or ] in bfrange array, got {:?}",
                                other
                            )))
                        }
                        None => {
                            return Err(CMapError::MissingKeyword("]".to_string()));
                        }
                    }
                }

                // Array length must equal hi-lo+1
                let expected_len = Self::range_length(&lo, &hi)?;
                if dst_strings.len() != expected_len {
                    return Err(CMapError::ArrayLengthMismatch);
                }

                // Add each mapping
                let mut current = lo.clone();
                for dst in dst_strings {
                    map.add_mapping(current.clone(), dst);
                    if !Self::increment_bytes(&mut current) {
                        break;
                    }
                }
            } else {
                // Contiguous form: <lo> <hi> <dst>
                let dst_hex = self.expect_hex_string()?;
                let mut dst = self.decode_utf16be(&dst_hex)?;

                // Expand range
                let mut current = lo.clone();
                loop {
                    map.add_mapping(current.clone(), dst.clone());
                    if current == hi {
                        break;
                    }
                    if !Self::increment_bytes(&mut current) {
                        break;
                    }
                    // Increment dst (only last codepoint for multi-codepoint dst)
                    Self::increment_dst(&mut dst);
                }
            }
        }

        // Expect endbfrange
        self.expect_keyword(b"endbfrange")?;

        Ok(())
    }

    /// Handle usecmap directive.
    ///
    /// For now, this just emits a diagnostic indicating that the named CMap
    /// is not available. Phase 2.3 will implement predefined CMap loading.
    fn handle_usecmap(&mut self) {
        // The name token should precede usecmap, but we've already consumed it.
        // Emit a diagnostic for now.
        self.diagnostics.push(Diagnostic::with_static(
            DiagCode::FontInvalidCmap,
            self.lexer.position(),
            "usecmap: predefined CMap loading not yet implemented (Phase 2.3)",
        ));
    }

    /// Decode a hex string as UTF-16BE.
    ///
    /// The hex string contains UTF-16BE encoded text. We decode it to a Vec<char>.
    /// Empty string returns empty vec.
    fn decode_utf16be(&mut self, bytes: &[u8]) -> Result<Vec<char>, CMapError> {
        if bytes.is_empty() {
            return Ok(Vec::new());
        }

        // UTF-16BE: pairs of bytes, big-endian
        let mut result = Vec::new();
        let mut i = 0;

        while i + 1 < bytes.len() {
            let hi = bytes[i] as u16;
            let lo = bytes[i + 1] as u16;
            let code_unit = (hi << 8) | lo;

            // decode_utf16 returns an iterator that yields Result<char, u16>
            for decoded in char::decode_utf16(std::iter::once(code_unit)) {
                match decoded {
                    Ok(c) => result.push(c),
                    Err(_) => {
                        // Unpaired surrogate - use replacement char
                        result.push('�');
                    }
                }
            }

            i += 2;
        }

        // Odd number of bytes - emit diagnostic but continue
        if i < bytes.len() {
            self.diagnostics.push(Diagnostic::with_static(
                DiagCode::FontInvalidCmap,
                self.lexer.position(),
                "UTF-16BE string has odd number of bytes",
            ));
        }

        Ok(result)
    }

    /// Expect an integer token.
    fn expect_integer(&mut self) -> Result<i64, CMapError> {
        match self.lexer.next_token() {
            Some(Token::Integer(n)) => Ok(n),
            Some(other) => Err(CMapError::UnexpectedToken(format!(
                "expected integer, got {:?}",
                other
            ))),
            None => Err(CMapError::MissingKeyword("integer".to_string())),
        }
    }

    /// Expect a hex string token (as Token::String).
    fn expect_hex_string(&mut self) -> Result<Vec<u8>, CMapError> {
        match self.lexer.next_token() {
            Some(Token::String(bytes)) => Ok(bytes),
            Some(Token::Keyword(kw)) if kw.is_empty() => {
                // Empty <> produces empty keyword - treat as empty hex string
                Ok(Vec::new())
            }
            Some(other) => Err(CMapError::UnexpectedToken(format!(
                "expected hex string, got {:?}",
                other
            ))),
            None => Err(CMapError::MissingKeyword("hex string".to_string())),
        }
    }

    /// Expect a specific keyword.
    fn expect_keyword(&mut self, expected: &[u8]) -> Result<(), CMapError> {
        match self.lexer.next_token() {
            Some(Token::Keyword(ref kw)) if kw == expected => Ok(()),
            Some(_other) => Err(CMapError::MissingKeyword(
                String::from_utf8_lossy(expected).to_string(),
            )),
            None => Err(CMapError::MissingKeyword(
                String::from_utf8_lossy(expected).to_string(),
            )),
        }
    }

    /// Skip tokens until we find the expected keyword.
    fn skip_to_keyword(&mut self, keyword: &[u8]) {
        while let Some(token) = self.lexer.next_token() {
            if let Token::Keyword(ref kw) = token {
                if kw == keyword {
                    break;
                }
            }
        }
    }

    /// Emit an error as a diagnostic.
    fn emit_error(&mut self, error: &CMapError) {
        self.diagnostics.push(Diagnostic::with_dynamic(
            DiagCode::FontInvalidCmap,
            self.lexer.position(),
            error.to_string(),
        ));
    }

    /// Calculate the length of a range (hi - lo + 1).
    ///
    /// This is the number of values in the range from lo to hi inclusive.
    fn range_length(lo: &[u8], hi: &[u8]) -> Result<usize, CMapError> {
        if lo.len() != hi.len() {
            // Different length sequences - use Hamming distance
            // This is unusual but technically valid
            return Ok(2); // Conservative estimate
        }

        // Calculate difference as big-endian integer
        let diff = if lo.len() <= 8 {
            // Fit in u64
            let lo_val = Self::bytes_to_u64(lo);
            let hi_val = Self::bytes_to_u64(hi);
            hi_val.saturating_sub(lo_val)
        } else {
            // Large sequences - use a safe default
            256
        };

        Ok((diff + 1) as usize)
    }

    /// Convert bytes to u64 (big-endian).
    fn bytes_to_u64(bytes: &[u8]) -> u64 {
        let mut result = 0u64;
        for &b in bytes {
            result = result * 256 + b as u64;
        }
        result
    }

    /// Increment a byte sequence (big-endian).
    ///
    /// Returns false if overflow occurs (all bytes were 0xFF).
    fn increment_bytes(bytes: &mut Vec<u8>) -> bool {
        for i in (0..bytes.len()).rev() {
            if bytes[i] < 0xFF {
                bytes[i] += 1;
                return true;
            } else {
                bytes[i] = 0;
            }
        }
        false // Overflow
    }

    /// Increment a destination string (increment only last codepoint).
    ///
    /// For multi-codepoint destinations (ligatures), only the last codepoint
    /// is incremented per spec.
    fn increment_dst(dst: &mut Vec<char>) {
        if let Some(last) = dst.last_mut() {
            *last = char::from_u32((*last as u32).wrapping_add(1)).unwrap_or('�');
        }
    }
}

/// Parse a ToUnicode CMap from raw bytes.
///
/// This is a convenience function that creates a parser and returns
/// just the map, discarding diagnostics.
pub fn parse_to_unicode(input: &[u8]) -> ToUnicodeMap {
    let parser = CMapParser::new(input);
    let (map, _diagnostics) = parser.parse();
    map
}

/// Parse a ToUnicode CMap from raw bytes with diagnostics.
///
/// Returns both the map and any diagnostics generated during parsing.
pub fn parse_to_unicode_with_diags(input: &[u8]) -> (ToUnicodeMap, Vec<Diagnostic>) {
    let parser = CMapParser::new(input);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_cmap() {
        let input = b"";
        let parser = CMapParser::new(input);
        let (map, _) = parser.parse();
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_single_bfchar() {
        // beginbfchar 1 <00> <0041> endbfchar
        let input = b"beginbfchar 1 <00> <0041> endbfchar";
        let parser = CMapParser::new(input);
        let (map, _) = parser.parse();

        assert_eq!(map.len(), 1);
        let result = map.lookup(&[0x00]);
        assert_eq!(result, Some(&['A'][..]));
    }

    #[test]
    fn test_parse_bfchar_ligature() {
        // beginbfchar 1 <00> <00660069> endbfchar
        // <00660069> is UTF-16BE for "fi" (U+0066 U+0069)
        let input = b"beginbfchar 1 <00> <00660069> endbfchar";
        let parser = CMapParser::new(input);
        let (map, _) = parser.parse();

        assert_eq!(map.len(), 1);
        let result = map.lookup(&[0x00]);
        assert_eq!(result, Some(&['f', 'i'][..]));
    }

    #[test]
    fn test_parse_bfchar_fb01_ligature() {
        // Acceptance criterion: beginbfchar <00> <FB01> parses
        // U+FB01 is the fi ligature single codepoint
        let input = b"beginbfchar 1 <00> <FB01> endbfchar";
        let parser = CMapParser::new(input);
        let (map, _) = parser.parse();

        assert_eq!(map.len(), 1);
        let result = map.lookup(&[0x00]);
        assert_eq!(result, Some(&['\u{FB01}'][..])); // fi ligature
    }

    #[test]
    fn test_parse_bfchar_multi_codepoint_expansion() {
        // Acceptance criterion: <00660069> multi-codepoint expands correctly
        let input = b"beginbfchar 1 <01> <00660069> endbfchar";
        let parser = CMapParser::new(input);
        let (map, _) = parser.parse();

        assert_eq!(map.len(), 1);
        let result = map.lookup(&[0x01]);
        assert_eq!(result, Some(&['f', 'i'][..]));
    }

    #[test]
    fn test_parse_bfrange_contiguous() {
        // Acceptance criterion: beginbfrange <0041> <005A> <0041> endbfrange
        // Maps A..=Z to U+0041..=U+005A
        let input = b"beginbfrange 1 <0041> <005A> <0041> endbfrange";
        let parser = CMapParser::new(input);
        let (map, _) = parser.parse();

        // Should have 26 mappings (A-Z)
        assert_eq!(map.len(), 26);

        // Check first and last
        assert_eq!(map.lookup(&[0x00, 0x41]), Some(&['A'][..]));
        assert_eq!(map.lookup(&[0x00, 0x5A]), Some(&['Z'][..]));
    }

    #[test]
    fn test_parse_bfrange_explicit_array() {
        // Acceptance criterion: beginbfrange <0001> <0003> [<FB01> <FB02> <FB03>] endbfrange
        // Maps codes 1,2,3 to ligatures fi, fl, ffi
        let input = b"beginbfrange 1 <0001> <0003> [<FB01> <FB02> <FB03>] endbfrange";
        let parser = CMapParser::new(input);
        let (map, _) = parser.parse();

        assert_eq!(map.len(), 3);
        assert_eq!(map.lookup(&[0x00, 0x01]), Some(&['\u{FB01}'][..])); // fi
        assert_eq!(map.lookup(&[0x00, 0x02]), Some(&['\u{FB02}'][..])); // fl
        assert_eq!(map.lookup(&[0x00, 0x03]), Some(&['\u{FB03}'][..])); // ffi
    }

    #[test]
    fn test_parse_comments() {
        // Acceptance criterion: Comment lines % foo ignored
        let input = b"% This is a comment\nbeginbfchar 1 <00> <0041> endbfchar\n% Another comment";
        let parser = CMapParser::new(input);
        let (map, diags) = parser.parse();

        assert_eq!(map.len(), 1);
        assert_eq!(map.lookup(&[0x00]), Some(&['A'][..]));
        // Comments should not produce diagnostics
        assert!(diags.is_empty());
    }

    #[test]
    fn test_parse_multiple_bfchar() {
        let input = b"beginbfchar 3 <00> <0041> <01> <0042> <02> <0043> endbfchar";
        let parser = CMapParser::new(input);
        let (map, _) = parser.parse();

        assert_eq!(map.len(), 3);
        assert_eq!(map.lookup(&[0x00]), Some(&['A'][..]));
        assert_eq!(map.lookup(&[0x01]), Some(&['B'][..]));
        assert_eq!(map.lookup(&[0x02]), Some(&['C'][..]));
    }

    #[test]
    fn test_parse_empty_destination() {
        // Empty destination <> should map to empty slice
        let input = b"beginbfchar 1 <00> <> endbfchar";
        let parser = CMapParser::new(input);
        let (map, _) = parser.parse();

        assert_eq!(map.len(), 1);
        assert_eq!(map.lookup(&[0x00]), Some(&[][..]));
    }

    #[test]
    fn test_parse_variable_width_source() {
        // Source codes with varying byte widths
        let input = b"beginbfchar 3 <00> <0041> <0001> <0042> <000001> <0043> endbfchar";
        let parser = CMapParser::new(input);
        let (map, _) = parser.parse();

        assert_eq!(map.len(), 3);
        assert_eq!(map.lookup(&[0x00]), Some(&['A'][..]));
        assert_eq!(map.lookup(&[0x00, 0x01]), Some(&['B'][..]));
        assert_eq!(map.lookup(&[0x00, 0x00, 0x01]), Some(&['C'][..]));
    }

    #[test]
    fn test_usecmap_emits_diagnostic() {
        let input = b"/Adobe-Japan1-UCS2 usecmap";
        let parser = CMapParser::new(input);
        let (map, diags) = parser.parse();

        assert!(map.is_empty());
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.message.as_ref().contains("usecmap")));
    }

    #[test]
    fn test_bfrange_multi_codepoint_dst_contiguous() {
        // Per spec note: contiguous bfrange where dst is multi-codepoint
        // Accept it, increment only the last codepoint
        let input = b"beginbfrange 1 <0001> <0002> <00660069> endbfrange";
        let parser = CMapParser::new(input);
        let (map, _) = parser.parse();

        assert_eq!(map.len(), 2);
        assert_eq!(map.lookup(&[0x00, 0x01]), Some(&['f', 'i'][..]));
        // Second entry: last codepoint incremented
        assert_eq!(map.lookup(&[0x00, 0x02]), Some(&['f', 'j'][..]));
    }

    #[test]
    fn test_invalid_utf16_produces_replacement() {
        // Unpaired surrogate in UTF-16BE
        let input = b"beginbfchar 1 <00> <D800> endbfchar"; // D800 is lone high surrogate
        let parser = CMapParser::new(input);
        let (map, _) = parser.parse();

        assert_eq!(map.len(), 1);
        // Should have replacement character
        let result = map.lookup(&[0x00]);
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_odd_length_utf16_emits_diagnostic() {
        // 5 hex digits -> 3 decoded bytes (odd), UTF-16BE requires even number of bytes
        let input = b"beginbfchar 1 <00> <00412> endbfchar";
        let parser = CMapParser::new(input);
        let (map, diags) = parser.parse();

        assert_eq!(map.len(), 1);
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.message.as_ref().contains("odd number of bytes")));
    }

    #[test]
    fn test_parse_convenience_function() {
        let input = b"beginbfchar 1 <00> <0041> endbfchar";
        let map = parse_to_unicode(input);

        assert_eq!(map.len(), 1);
        assert_eq!(map.lookup(&[0x00]), Some(&['A'][..]));
    }

    #[test]
    fn test_bfrange_array_length_mismatch() {
        // Array with wrong length for the range
        let input = b"beginbfrange 1 <0001> <0003> [<FB01> <FB02>] endbfrange"; // 3 expected, 2 provided
        let parser = CMapParser::new(input);
        let (map, diags) = parser.parse();

        // Should fail and emit diagnostic
        assert!(map.is_empty() || map.len() < 3);
        assert!(!diags.is_empty());
    }

    #[test]
    fn test_bfrange_invalid_range() {
        // lo > hi
        let input = b"beginbfrange 1 <0005> <0001> <0041> endbfrange";
        let parser = CMapParser::new(input);
        let (map, diags) = parser.parse();

        // Should fail and emit diagnostic
        assert!(map.is_empty());
        assert!(!diags.is_empty());
    }
}
