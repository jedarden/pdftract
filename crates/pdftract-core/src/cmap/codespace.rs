//! Codespace range parser for CMap streams.
//!
//! This module implements parsing of the `begincodespacerange` / `endcodespacerange`
//! PostScript blocks in CMap streams. Codespace ranges define the valid byte-width
//! boundaries for character codes in multi-byte encodings.
//!
//! # Syntax
//!
//! PostScript CMap codespace range syntax:
//! ```text
//! N begincodespacerange
//!   <lo1> <hi1>
//!   <lo2> <hi2>
//!   ...
//! endcodespacerange
//! ```
//!
//! Each entry consists of two hex strings of equal byte width (1-4 bytes).
//!
//! # Example
//!
//! ```text
//! 2 begincodespacerange
//!   <00> <7F>
//!   <8000> <FFFF>
//! endcodespacerange
//! ```
//!
//! Defines two ranges:
//! - 1-byte range: 0x00..=0x7F
//! - 2-byte range: 0x8000..=0xFFFF

use std::fmt;

use crate::{emit, diagnostics::DiagCode};

/// A single codespace range.
///
/// Defines a contiguous range of valid character codes with a fixed byte width.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodespaceRange {
    /// Low bound of the range (inclusive), stored in big-endian byte order.
    pub lo: [u8; 4],
    /// High bound of the range (inclusive), stored in big-endian byte order.
    pub hi: [u8; 4],
    /// Byte width of this range (1, 2, 3, or 4).
    pub width: u8,
}

impl CodespaceRange {
    /// Create a new codespace range.
    ///
    /// MARKER: CMAP entry creation point - this is where individual codespace
    /// range entries are created for multi-byte CJK encodings. Called from
    /// parse_codespace_block(). See notes/bf-e4uvb-child-1.md for documentation.
    ///
    /// # Panics
    ///
    /// Panics if width is not 1, 2, 3, or 4, or if lo and hi have mismatched widths.
    pub fn new(lo: [u8; 4], hi: [u8; 4], width: u8) -> Self {
        assert!(width >= 1 && width <= 4, "width must be 1-4");
        assert!(width as usize <= lo.len() && width as usize <= hi.len());
        Self { lo, hi, width }
    }

    /// Check if a byte sequence falls within this codespace range.
    ///
    /// Returns true if the sequence's byte width matches this range's width
    /// and its value falls within [lo, hi] inclusive.
    pub fn contains(&self, bytes: &[u8]) -> bool {
        if bytes.len() != self.width as usize {
            return false;
        }

        // Compare bytes up to width
        for i in 0..self.width as usize {
            let b = bytes[i];
            if b < self.lo[i] || b > self.hi[i] {
                return false;
            }
        }

        true
    }

    /// Get the low bound as a slice (only valid bytes up to width).
    pub fn lo_slice(&self) -> &[u8] {
        &self.lo[..self.width as usize]
    }

    /// Get the high bound as a slice (only valid bytes up to width).
    pub fn hi_slice(&self) -> &[u8] {
        &self.hi[..self.width as usize]
    }
}

impl fmt::Display for CodespaceRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lo_hex: String = self.lo_slice().iter().map(|b| format!("{:02X}", b)).collect();
        let hi_hex: String = self.hi_slice().iter().map(|b| format!("{:02X}", b)).collect();
        write!(
            f,
            "<{}> <{}> ({} byte{})",
            lo_hex,
            hi_hex,
            self.width,
            if self.width == 1 { "" } else { "s" }
        )
    }
}

/// Collection of codespace ranges from a CMap.
///
/// Most CMaps define 1-8 ranges. Predefined CMaps typically define:
/// - 1-byte ASCII range: \`<00> <7F>\`
/// - 2-byte CJK range: \`<8000> <FFFF\>\` (or similar)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodespaceRanges {
    /// The ranges in this CMap.
    pub ranges: smallvec::SmallVec<[CodespaceRange; 8]>,
}

impl CodespaceRanges {
    /// Create an empty codespace ranges collection.
    pub fn new() -> Self {
        Self {
            ranges: smallvec::SmallVec::new(),
        }
    }

    /// Add a codespace range to this collection.
    pub fn push(&mut self, range: CodespaceRange) {
        self.ranges.push(range);
    }

    /// Check if this collection is empty.
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Get the number of ranges in this collection.
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Find which codespace range a byte sequence falls into.
    ///
    /// Returns the index of the matching range, or None if no range matches.
    pub fn find_range(&self, bytes: &[u8]) -> Option<usize> {
        self.ranges
            .iter()
            .position(|range| range.contains(bytes))
    }

    /// Get all ranges in this collection.
    pub fn as_slice(&self) -> &[CodespaceRange] {
        &self.ranges
    }
}

impl Default for CodespaceRanges {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CodespaceRanges {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffix = if self.len() == 1 { "" } else { "s" };
        writeln!(f, "CodespaceRanges ({} range{}):", self.len(), suffix)?;
        for range in &self.ranges {
            writeln!(f, "  {}", range)?;
        }
        Ok(())
    }
}

/// Result type for codespace parsing.
pub type CodespaceResult<T> = Result<T, CodespaceError>;

/// Errors that can occur during codespace range parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodespaceError {
    /// Invalid hex string format.
    InvalidHexString(String),
    /// Width mismatch between lo and hi bounds.
    WidthMismatch {
        /// Width of the lo bound.
        lo_width: usize,
        /// Width of the hi bound.
        hi_width: usize,
    },
    /// Invalid width (not 1, 2, 3, or 4).
    InvalidWidth(usize),
    /// Unexpected token in codespace block.
    UnexpectedToken(String),
}

impl fmt::Display for CodespaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodespaceError::InvalidHexString(msg) => write!(f, "invalid hex string: {}", msg),
            CodespaceError::WidthMismatch { lo_width, hi_width } => {
                write!(f, "width mismatch: lo has {} bytes, hi has {} bytes", lo_width, hi_width)
            }
            CodespaceError::InvalidWidth(width) => write!(f, "invalid width: {} (must be 1-4)", width),
            CodespaceError::UnexpectedToken(msg) => write!(f, "unexpected token: {}", msg),
        }
    }
}

impl std::error::Error for CodespaceError {}

/// Codespace range parser for CMap streams.
///
/// Parses PostScript-style `begincodespacerange` / `endcodespacerange` blocks
/// and extracts the byte-width boundaries used for multi-byte tokenization.
pub struct CodespaceParser<'a> {
    input: &'a [u8],
    position: usize,
    pending_count: Option<i64>,
    diagnostics: Vec<crate::diagnostics::Diagnostic>,
}

impl<'a> CodespaceParser<'a> {
    /// Create a new codespace parser for the given input bytes.
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            position: 0,
            pending_count: None,
            diagnostics: Vec::new(),
        }
    }

    /// Parse the codespace ranges from the input.
    ///
    /// Returns the parsed ranges along with any diagnostics generated during parsing.
    pub fn parse(mut self) -> (CodespaceRanges, Vec<crate::diagnostics::Diagnostic>) {
        let mut ranges = CodespaceRanges::new();

        while let Some(token) = self.next_token() {
            match token {
                Token::Eof => break,
                Token::Integer(n) => {
                    // Store integer - may be a count before begincodespacerange
                    self.pending_count = Some(n);
                }
                Token::Keyword(ref kw) => {
                    match kw.as_slice() {
                        b"begincodespacerange" => {
                            if let Err(e) = self.parse_codespace_block(&mut ranges) {
                                self.emit_error(&e);
                                // Recovery: skip to endcodespacerange
                                self.skip_to_keyword(b"endcodespacerange");
                            }
                            // Clear pending count in case it wasn't used
                            self.pending_count = None;
                        }
                        b"endcodespacerange" => {
                            // Unexpected - should have been consumed by parse_codespace_block
                            self.diagnostics.push(crate::diagnostics::Diagnostic::with_dynamic(
                                DiagCode::CmapInvalidCodespace,
                                self.position as u64,
                                "Unbalanced codespace block: endcodespacerange without begincodespacerange".to_string(),
                            ));
                            self.pending_count = None;
                        }
                        _ => {
                            // Unknown keyword - clear pending count (not for us)
                            self.pending_count = None;
                        }
                    }
                }
                _ => {
                    // Unexpected token - clear pending count
                    self.pending_count = None;
                }
            }
        }

        (ranges, self.diagnostics)
    }

    /// Parse a begincodespacerange...endcodespacerange block.
    fn parse_codespace_block(&mut self, ranges: &mut CodespaceRanges) -> Result<(), CodespaceError> {
        // Read count - may be pending (from before keyword) or after keyword
        let count = match self.pending_count.take() {
            Some(n) => {
                if n < 0 {
                    return Err(CodespaceError::UnexpectedToken(
                        "negative codespace range count".to_string(),
                    ));
                }
                n as usize
            }
            None => {
                let n = self.expect_integer()?;
                if n < 0 {
                    return Err(CodespaceError::UnexpectedToken(
                        "negative codespace range count".to_string(),
                    ));
                }
                n as usize
            }
        };

        // Read count pairs of <lo> <hi>
        for _ in 0..count {
            let lo = match self.expect_hex_string() {
                Ok(s) => s,
                Err(_) => {
                    // Failed to read lo - skip to endcodespacerange
                    emit!(self.diagnostics, CmapInvalidCodespace);
                    self.skip_to_keyword(b"endcodespacerange");
                    break;
                }
            };

            let hi = match self.expect_hex_string() {
                Ok(s) => s,
                Err(_) => {
                    // Failed to read hi - skip to endcodespacerange
                    emit!(self.diagnostics, CmapInvalidCodespace);
                    self.skip_to_keyword(b"endcodespacerange");
                    break;
                }
            };

            // Validate width
            if lo.len() != hi.len() {
                emit!(self.diagnostics, CmapInvalidCodespace);
                // Skip this invalid range and continue to the next
                continue;
            }

            let width = lo.len();
            if width < 1 || width > 4 {
                emit!(self.diagnostics, CmapInvalidCodespace);
                // Skip this invalid range and continue to the next
                continue;
            }

            // Create range with 4-byte arrays
            let mut lo_arr = [0u8; 4];
            let mut hi_arr = [0u8; 4];
            for (i, &b) in lo.iter().enumerate() {
                lo_arr[i] = b;
            }
            for (i, &b) in hi.iter().enumerate() {
                hi_arr[i] = b;
            }

            // MARKER: CMAP entry creation point - codespace range creation for CJK encodings.
            // See notes/bf-e4uvb-child-1.md for documentation.
            ranges.push(CodespaceRange::new(lo_arr, hi_arr, width as u8));
        }

        // Expect endcodespacerange
        self.expect_keyword(b"endcodespacerange")?;

        Ok(())
    }

    /// Get the next token from the input.
    fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();

        if self.position >= self.input.len() {
            return Some(Token::Eof);
        }

        let byte = self.input[self.position];

        match byte {
            b'<' => {
                // Hex string or dictionary marker
                if self.position + 1 < self.input.len() && self.input[self.position + 1] == b'<' {
                    self.position += 2;
                    Some(Token::DictStart)
                } else {
                    self.parse_hex_string().map(Token::String)
                }
            }
            b'>' => {
                // Dictionary end
                if self.position + 1 < self.input.len() && self.input[self.position + 1] == b'>' {
                    self.position += 2;
                    Some(Token::DictEnd)
                } else {
                    // Lone > - treat as unexpected
                    self.position += 1;
                    Some(Token::Unexpected(byte))
                }
            }
            b'/' => {
                // Name (skip for codespace parsing)
                self.parse_name();
                self.next_token()
            }
            b'0'..=b'9' | b'-' => {
                // Integer
                self.parse_integer().map(Token::Integer)
            }
            b'%' => {
                // Comment - skip to end of line
                while self.position < self.input.len() && self.input[self.position] != b'\n' {
                    self.position += 1;
                }
                self.next_token()
            }
            b'a'..=b'z' | b'A'..=b'Z' => {
                // Keyword
                self.parse_keyword().map(Token::Keyword)
            }
            _ => {
                // Unexpected byte
                self.position += 1;
                Some(Token::Unexpected(byte))
            }
        }
    }

    /// Parse a hex string <...>.
    fn parse_hex_string(&mut self) -> Option<Vec<u8>> {
        if self.position >= self.input.len() || self.input[self.position] != b'<' {
            return None;
        }
        self.position += 1; // skip <

        // Check for empty string <>
        if self.position < self.input.len() && self.input[self.position] == b'>' {
            self.position += 1;
            return Some(Vec::new());
        }

        let mut bytes = Vec::new();
        let mut current = 0u8;
        let mut nibble = 0;

        while self.position < self.input.len() {
            let byte = self.input[self.position];
            self.position += 1;

            if byte == b'>' {
                if nibble == 1 {
                    bytes.push(current);
                }
                break;
            }

            // Skip whitespace in hex string
            if byte.is_ascii_whitespace() {
                continue;
            }

            // Parse hex nibble
            let nibble_value = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => {
                    // Invalid hex - emit diagnostic and skip
                    emit!(self.diagnostics, CmapInvalidCodespace);
                    continue;
                }
            };

            if nibble == 0 {
                current = nibble_value << 4;
                nibble = 1;
            } else {
                current |= nibble_value;
                bytes.push(current);
                current = 0;
                nibble = 0;
            }
        }

        Some(bytes)
    }

    /// Parse an integer.
    fn parse_integer(&mut self) -> Option<i64> {
        let start = self.position;

        // Handle optional negative sign
        if self.position < self.input.len() && self.input[self.position] == b'-' {
            self.position += 1;
        }

        // Parse digits
        while self.position < self.input.len() && self.input[self.position].is_ascii_digit() {
            self.position += 1;
        }

        if self.position == start {
            return None;
        }

        let s = std::str::from_utf8(&self.input[start..self.position]).ok()?;
        s.parse().ok()
    }

    /// Parse a keyword (sequence of letters).
    fn parse_keyword(&mut self) -> Option<Vec<u8>> {
        let start = self.position;

        while self.position < self.input.len() {
            let byte = self.input[self.position];
            if byte.is_ascii_alphabetic() {
                self.position += 1;
            } else {
                break;
            }
        }

        if self.position > start {
            Some(self.input[start..self.position].to_vec())
        } else {
            None
        }
    }

    /// Parse and skip a name (/Name).
    fn parse_name(&mut self) {
        if self.position < self.input.len() && self.input[self.position] == b'/' {
            self.position += 1;
            // Skip to next whitespace or delimiter
            while self.position < self.input.len() && !self.input[self.position].is_ascii_whitespace() && self.input[self.position] != b'/' && self.input[self.position] != b'<' && self.input[self.position] != b'>' {
                self.position += 1;
            }
        }
    }

    /// Skip whitespace.
    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() && self.input[self.position].is_ascii_whitespace() {
            self.position += 1;
        }
    }

    /// Expect an integer token.
    fn expect_integer(&mut self) -> Result<i64, CodespaceError> {
        match self.next_token() {
            Some(Token::Integer(n)) => Ok(n),
            Some(other) => Err(CodespaceError::UnexpectedToken(format!(
                "expected integer, got {:?}",
                other
            ))),
            None => Err(CodespaceError::UnexpectedToken("expected integer".to_string())),
        }
    }

    /// Expect a hex string token.
    fn expect_hex_string(&mut self) -> Result<Vec<u8>, CodespaceError> {
        match self.next_token() {
            Some(Token::String(bytes)) => Ok(bytes),
            Some(other) => Err(CodespaceError::UnexpectedToken(format!(
                "expected hex string, got {:?}",
                other
            ))),
            None => Err(CodespaceError::UnexpectedToken("expected hex string".to_string())),
        }
    }

    /// Expect a specific keyword.
    fn expect_keyword(&mut self, expected: &[u8]) -> Result<(), CodespaceError> {
        match self.next_token() {
            Some(Token::Keyword(ref kw)) if kw == expected => Ok(()),
            Some(_other) => Err(CodespaceError::UnexpectedToken(format!(
                "expected keyword {}",
                String::from_utf8_lossy(expected)
            ))),
            None => Err(CodespaceError::UnexpectedToken(format!(
                "expected keyword {}",
                String::from_utf8_lossy(expected)
            ))),
        }
    }

    /// Skip tokens until we find the expected keyword.
    fn skip_to_keyword(&mut self, keyword: &[u8]) {
        while let Some(token) = self.next_token() {
            if let Token::Keyword(ref kw) = token {
                if kw == keyword {
                    break;
                }
            }
        }
    }

    /// Emit an error as a diagnostic.
    fn emit_error(&mut self, error: &CodespaceError) {
        self.diagnostics.push(crate::diagnostics::Diagnostic::with_dynamic(
            DiagCode::CmapInvalidCodespace,
            self.position as u64,
            error.to_string(),
        ));
    }
}

/// Token produced by the codespace lexer.
#[derive(Debug)]
enum Token {
    /// End of input
    Eof,
    /// Hex string contents (without < > delimiters)
    String(Vec<u8>),
    /// Integer value
    Integer(i64),
    /// Keyword (e.g., begincodespacerange)
    Keyword(Vec<u8>),
    /// Dictionary start (<<)
    DictStart,
    /// Dictionary end (>>)
    DictEnd,
    /// Unexpected byte
    Unexpected(u8),
}

/// Parse codespace ranges from raw CMap bytes.
///
/// This is a convenience function that creates a parser and returns
/// just the ranges, discarding diagnostics.
pub fn parse_codespace_ranges(input: &[u8]) -> CodespaceRanges {
    let parser = CodespaceParser::new(input);
    let (ranges, _diagnostics) = parser.parse();
    ranges
}

/// Parse codespace ranges from raw CMap bytes with diagnostics.
///
/// Returns both the ranges and any diagnostics generated during parsing.
pub fn parse_codespace_ranges_with_diags(input: &[u8]) -> (CodespaceRanges, Vec<crate::diagnostics::Diagnostic>) {
    let parser = CodespaceParser::new(input);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_range_1_byte() {
        let input = b"1 begincodespacerange\n<00> <7F>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, diags) = parser.parse();

        assert_eq!(ranges.len(), 1);
        assert!(diags.is_empty());

        let range = &ranges.ranges[0];
        assert_eq!(range.width, 1);
        assert_eq!(range.lo_slice(), &[0x00]);
        assert_eq!(range.hi_slice(), &[0x7F]);
    }

    #[test]
    fn test_parse_two_ranges_mixed_width() {
        // Acceptance criterion: <00> <7F> <8000> <FFFF> in one block → 2 ranges
        let input = b"2 begincodespacerange\n<00> <7F>\n<8000> <FFFF>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, diags) = parser.parse();

        assert_eq!(ranges.len(), 2);
        assert!(diags.is_empty());

        // First range: 1-byte
        assert_eq!(ranges.ranges[0].width, 1);
        assert_eq!(ranges.ranges[0].lo_slice(), &[0x00]);
        assert_eq!(ranges.ranges[0].hi_slice(), &[0x7F]);

        // Second range: 2-byte
        assert_eq!(ranges.ranges[1].width, 2);
        assert_eq!(ranges.ranges[1].lo_slice(), &[0x80, 0x00]);
        assert_eq!(ranges.ranges[1].hi_slice(), &[0xFF, 0xFF]);
    }

    #[test]
    fn test_width_inference() {
        // Acceptance criterion: 2-char hex → width=1; 4-char hex → width=2
        let input = b"2 begincodespacerange\n<C0> <FF>\n<8140> <FEFE>\nendcodespacerange";
        let ranges = parse_codespace_ranges(input);

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges.ranges[0].width, 1);
        assert_eq!(ranges.ranges[1].width, 2);
    }

    #[test]
    fn test_case_insensitive_hex() {
        // Acceptance criterion: <C0> and <c0> equivalent
        let input = b"2 begincodespacerange\n<C0> <FF>\n<c0> <ff>\nendcodespacerange";
        let ranges = parse_codespace_ranges(input);

        assert_eq!(ranges.len(), 2);
        // Both ranges should parse identically
        assert_eq!(ranges.ranges[0].lo_slice(), ranges.ranges[1].lo_slice());
        assert_eq!(ranges.ranges[0].hi_slice(), ranges.ranges[1].hi_slice());
    }

    #[test]
    fn test_width_mismatch_emits_diagnostic() {
        // Acceptance criterion: mismatched lo/hi width → diagnostic + skipped
        let input = b"1 begincodespacerange\n<00> <FFFF>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, diags) = parser.parse();

        // Should have diagnostic and empty ranges (recovery)
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.code == DiagCode::CmapInvalidCodespace));
        // The malformed range should be skipped
        assert_eq!(ranges.len(), 0);
    }

    #[test]
    fn test_empty_cmap() {
        // Acceptance criterion: empty CMap → empty ranges
        let input = b"";
        let ranges = parse_codespace_ranges(input);

        assert!(ranges.is_empty());
    }

    #[test]
    fn test_jis_lead_trail_pattern() {
        // JIS 2-byte pattern example
        let input = b"1 begincodespacerange\n<8140> <FEFE>\nendcodespacerange";
        let ranges = parse_codespace_ranges(input);

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.ranges[0].width, 2);
        assert_eq!(ranges.ranges[0].lo_slice(), &[0x81, 0x40]);
        assert_eq!(ranges.ranges[0].hi_slice(), &[0xFE, 0xFE]);
    }

    #[test]
    fn test_codespace_range_contains() {
        let range = CodespaceRange::new([0x00, 0, 0, 0], [0x7F, 0, 0, 0], 1);

        // Valid bytes in range
        assert!(range.contains(&[0x00]));
        assert!(range.contains(&[0x40]));
        assert!(range.contains(&[0x7F]));

        // Outside range
        assert!(!range.contains(&[0x80]));
        assert!(!range.contains(&[0xFF]));

        // Wrong width
        assert!(!range.contains(&[]));
        assert!(!range.contains(&[0x00, 0x00]));
    }

    #[test]
    fn test_codespace_range_contains_2_byte() {
        let range = CodespaceRange::new([0x80, 0x00, 0, 0], [0xFF, 0xFF, 0, 0], 2);

        // Valid bytes in range
        assert!(range.contains(&[0x80, 0x00]));
        assert!(range.contains(&[0xA0, 0xA0]));
        assert!(range.contains(&[0xFF, 0xFF]));

        // Outside range
        assert!(!range.contains(&[0x00, 0x00]));
        assert!(!range.contains(&[0x7F, 0xFF]));

        // Wrong width
        assert!(!range.contains(&[0x80]));
        assert!(!range.contains(&[0x80, 0x00, 0x00]));
    }

    #[test]
    fn test_find_range() {
        let mut ranges = CodespaceRanges::new();
        ranges.push(CodespaceRange::new([0x00, 0, 0, 0], [0x7F, 0, 0, 0], 1));
        ranges.push(CodespaceRange::new([0x80, 0x00, 0, 0], [0xFF, 0xFF, 0, 0], 2));

        // 1-byte sequence
        assert_eq!(ranges.find_range(&[0x40]), Some(0));
        assert_eq!(ranges.find_range(&[0x80]), None);

        // 2-byte sequence
        assert_eq!(ranges.find_range(&[0x80, 0x00]), Some(1));
        assert_eq!(ranges.find_range(&[0x00, 0x00]), None);
    }

    #[test]
    fn test_invalid_hex_emits_diagnostic() {
        // Invalid hex characters in string
        let input = b"1 begincodespacerange\n<XG> <FF>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, diags) = parser.parse();

        // Should have diagnostic
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.code == DiagCode::CmapInvalidCodespace));
    }

    #[test]
    fn test_empty_hex_string() {
        // Empty hex string <>
        let input = b"1 begincodespacerange\n<> <>\nendcodespacerange";
        let ranges = parse_codespace_ranges(input);

        // Empty strings parse as 0 bytes, width 0 is invalid
        // This should produce a diagnostic
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_3_byte_range() {
        // 3-byte range (valid per spec)
        let input = b"1 begincodespacerange\n<800000> <FFFFFF>\nendcodespacerange";
        let ranges = parse_codespace_ranges(input);

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.ranges[0].width, 3);
        assert_eq!(ranges.ranges[0].lo_slice(), &[0x80, 0x00, 0x00]);
        assert_eq!(ranges.ranges[0].hi_slice(), &[0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_4_byte_range() {
        // 4-byte range (max valid width)
        let input = b"1 begincodespacerange\n<80000000> <FFFFFFFF>\nendcodespacerange";
        let ranges = parse_codespace_ranges(input);

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.ranges[0].width, 4);
        assert_eq!(ranges.ranges[0].lo_slice(), &[0x80, 0x00, 0x00, 0x00]);
        assert_eq!(ranges.ranges[0].hi_slice(), &[0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_comments_ignored() {
        // Comments should be ignored
        let input = b"% This is a comment\n1 begincodespacerange\n% Another comment\n<00> <7F>\nendcodespacerange";
        let ranges = parse_codespace_ranges(input);

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.ranges[0].width, 1);
    }

    #[test]
    fn test_whitespace_variations() {
        // Various whitespace forms
        let input = b"1 begincodespacerace <00> <7F> endcodespacerace";
        // Note: typo in keyword would cause this to fail - let's fix it
        let input = b"1 begincodespacerange\t<00>\t<7F>\nendcodespacerange";
        let ranges = parse_codespace_ranges(input);

        assert_eq!(ranges.len(), 1);
    }

    #[test]
    fn test_recovery_after_invalid_range() {
        // First range is invalid, second is valid
        let input = b"2 begincodespacerange\n<00> <FFFF>\n<00> <7F>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, diags) = parser.parse();

        // Should have diagnostic for first range
        assert!(!diags.is_empty());
        // Should skip first range but continue to parse second
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.ranges[0].width, 1);
    }

    #[test]
    fn test_display() {
        let ranges = CodespaceRanges {
            ranges: smallvec::smallvec![
                CodespaceRange::new([0x00, 0, 0, 0], [0x7F, 0, 0, 0], 1),
                CodespaceRange::new([0x80, 0x00, 0, 0], [0xFF, 0xFF, 0, 0], 2),
            ],
        };

        let display = format!("{}", ranges);
        assert!(display.contains("CodespaceRanges"));
        assert!(display.contains("2 ranges"));
    }

    #[test]
    fn test_identity_h_cmap() {
        // Identity-H CMap has specific codespace ranges
        // Most commonly: <00> <FF> for 1-byte and <0100> <FFFF> for 2-byte
        let input = b"2 begincodespacerange\n<00> <FF>\n<0100> <FFFF>\nendcodespacerange";
        let ranges = parse_codespace_ranges(input);

        assert_eq!(ranges.len(), 2);

        // 1-byte range covers all single bytes
        assert_eq!(ranges.ranges[0].width, 1);
        assert!(ranges.ranges[0].contains(&[0x00]));
        assert!(ranges.ranges[0].contains(&[0xFF]));

        // 2-byte range covers 0x0100-0xFFFF
        assert_eq!(ranges.ranges[1].width, 2);
        assert!(ranges.ranges[1].contains(&[0x01, 0x00]));
        assert!(ranges.ranges[1].contains(&[0xFF, 0xFF]));
    }
}
