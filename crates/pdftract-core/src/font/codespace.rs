//! CMap codespace range parser.
//!
//! This module implements parsing of the `begincodespacerange` / `endcodespacerange`
//! PostScript blocks in CMap streams. Codespace ranges define the legal byte-width
//! boundaries for character codes in a CMap.
//!
//! # Codespace ranges
//!
//! A codespace range defines a contiguous range of character codes with the same
//! byte width. For example:
//! - `<00> <7F>` → 1-byte range covering 0x00..=0x7F
//! - `<8000> <FFFF>` → 2-byte range
//! - `<8140> <FEFE>` → JIS lead/trail 2-byte pattern
//!
//! # PostScript syntax
//!
//! ```text
//! N begincodespacerange
//!   <lo1> <hi1>
//!   <lo2> <hi2>
//!   ...
//! endcodespacerange
//! ```
//!
//! Each entry is two hex strings of equal byte width (1, 2, 3, or 4 bytes after hex decode).

use smallvec::SmallVec;

use crate::diagnostics::{DiagCode, Diagnostic};

/// A single codespace range.
///
/// Defines a contiguous range of character codes with a fixed byte width.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodespaceRange {
    /// Low bound of the range (inclusive), stored big-endian in 4 bytes
    pub lo: [u8; 4],
    /// High bound of the range (inclusive), stored big-endian in 4 bytes
    pub hi: [u8; 4],
    /// Byte width of this range (1, 2, 3, or 4)
    pub width: u8,
}

impl CodespaceRange {
    /// Create a new codespace range.
    ///
    /// # Arguments
    ///
    /// * `lo` - Low bound bytes (big-endian)
    /// * `hi` - High bound bytes (big-endian)
    ///
    /// # Returns
    ///
    /// `None` if lo and hi have different lengths or if width is not 1-4.
    pub fn new(lo: Vec<u8>, hi: Vec<u8>) -> Option<Self> {
        if lo.len() != hi.len() {
            return None;
        }
        let width = lo.len();
        if !(1..=4).contains(&width) {
            return None;
        }

        // Convert to 4-byte big-endian arrays
        let mut lo_arr = [0u8; 4];
        let mut hi_arr = [0u8; 4];
        let offset = 4 - width;
        lo_arr[offset..].copy_from_slice(&lo);
        hi_arr[offset..].copy_from_slice(&hi);

        Some(CodespaceRange {
            lo: lo_arr,
            hi: hi_arr,
            width: width as u8,
        })
    }

    /// Get the low bound as a slice (without leading zeros).
    pub fn lo_slice(&self) -> &[u8] {
        let offset = 4 - self.width as usize;
        &self.lo[offset..]
    }

    /// Get the high bound as a slice (without leading zeros).
    pub fn hi_slice(&self) -> &[u8] {
        let offset = 4 - self.width as usize;
        &self.hi[offset..]
    }
}

/// Collection of codespace ranges from a CMap.
///
/// Most predefined CMaps (Identity-H/V, UTF-16 variants) use 1-byte ASCII
/// plus 2-byte CJK ranges.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodespaceRanges {
    /// The ranges in this collection (typically 1-8 entries)
    pub ranges: SmallVec<[CodespaceRange; 8]>,
}

impl CodespaceRanges {
    /// Create a new empty collection.
    pub fn new() -> Self {
        Self {
            ranges: SmallVec::new(),
        }
    }

    /// Add a range to the collection.
    pub fn add(&mut self, range: CodespaceRange) {
        self.ranges.push(range);
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Get the number of ranges.
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Find the matching range for a given byte sequence.
    ///
    /// Returns the range if the byte sequence falls within it, considering width.
    pub fn find_range(&self, code: &[u8]) -> Option<&CodespaceRange> {
        for range in &self.ranges {
            let width = range.width as usize;
            if code.len() == width {
                let offset = 4 - width;
                let lo = &range.lo[offset..];
                let hi = &range.hi[offset..];
                if code >= lo && code <= hi {
                    return Some(range);
                }
            }
        }
        None
    }
}

/// Error that can occur during codespace range parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodespaceError {
    /// Unexpected token in CMap stream.
    UnexpectedToken(String),
    /// Invalid hex string format.
    InvalidHexString(String),
    /// Missing expected keyword (e.g., endcodespacerange).
    MissingKeyword(String),
    /// Width mismatch between lo and hi bounds.
    WidthMismatch,
}

impl std::fmt::Display for CodespaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodespaceError::UnexpectedToken(msg) => write!(f, "unexpected token: {}", msg),
            CodespaceError::InvalidHexString(msg) => write!(f, "invalid hex string: {}", msg),
            CodespaceError::MissingKeyword(kw) => write!(f, "missing expected keyword: {}", kw),
            CodespaceError::WidthMismatch => write!(f, "codespace range lo/hi width mismatch"),
        }
    }
}

/// Codespace range parser.
///
/// Parses a PostScript CMap program and extracts codespace ranges.
pub struct CodespaceParser<'a> {
    /// Input bytes
    input: &'a [u8],
    /// Current position
    pos: usize,
    /// Accumulated diagnostics
    diagnostics: Vec<Diagnostic>,
}

impl<'a> CodespaceParser<'a> {
    /// Create a new codespace parser for the given input bytes.
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    /// Parse the codespace ranges from the input.
    ///
    /// Returns the parsed ranges and any diagnostics generated during parsing.
    pub fn parse(mut self) -> (CodespaceRanges, Vec<Diagnostic>) {
        let mut ranges = CodespaceRanges::new();
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10000; // Safety limit for infinite loop detection

        while self.pos < self.input.len() {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                panic!("CodespaceParser::parse exceeded maximum iterations ({}), likely infinite loop. Current position: {}, input length: {}",
                       MAX_ITERATIONS, self.pos, self.input.len());
            }

            let pos_before = self.pos;

            // Skip whitespace and comments
            self.skip_whitespace_and_comments();

            // Check for EOF
            if self.pos >= self.input.len() {
                break;
            }

            // Try to parse begincodespacerange
            if self.try_keyword(b"begincodespacerange") {
                if let Err(e) = self.parse_codespace_block(&mut ranges) {
                    self.emit_error(&e);
                    // Attempt recovery: skip to endcodespacerange
                    self.skip_to_keyword(b"endcodespacerange");
                }
                continue;
            }

            // Skip unknown tokens
            self.skip_token();

            // Safety check: ensure we made progress; if not, advance by one to prevent infinite loop
            if self.pos == pos_before && self.pos < self.input.len() {
                eprintln!("Warning: CodespaceParser made no progress, forcing position advance from {} to {}", self.pos, self.pos + 1);
                self.pos += 1;
            }
        }

        (ranges, self.diagnostics)
    }

    /// Parse a begincodespacerange...endcodespacerange block.
    fn parse_codespace_block(
        &mut self,
        ranges: &mut CodespaceRanges,
    ) -> Result<(), CodespaceError> {
        // Read count (optional in some CMaps, but standard requires it)
        let count = if let Ok(n) = self.try_integer() {
            if n < 0 {
                return Err(CodespaceError::UnexpectedToken(
                    "negative range count".to_string(),
                ));
            }
            n as usize
        } else {
            // No count - parse until endcodespacerange
            usize::MAX
        };

        let mut parsed = 0;
        while parsed < count {
            self.skip_whitespace_and_comments();

            // Check for endcodespacerange
            if self.try_keyword(b"endcodespacerange") {
                break;
            }

            // Parse lo hex string
            let lo = self.expect_hex_string()?;

            // Skip whitespace
            self.skip_whitespace();

            // Parse hi hex string
            let hi = self.expect_hex_string()?;

            // Create range
            // MARKER: CMAP entry creation point - codespace range definitions.
            // See notes/bf-e4uvb-child-1.md for documentation.
            if let Some(range) = CodespaceRange::new(lo, hi) {
                ranges.add(range);
                parsed += 1;
            } else {
                // Width mismatch or invalid width
                self.diagnostics.push(Diagnostic::with_dynamic(
                    DiagCode::FontInvalidCmap,
                    self.pos as u64,
                    format!("codespace range lo/hi width mismatch or invalid width"),
                ));
                // Continue parsing other ranges
                parsed += 1;
            }

            self.skip_whitespace_and_comments();
        }

        // If we had a count, expect endcodespacerange
        if count != usize::MAX && !self.try_keyword(b"endcodespacerange") {
            return Err(CodespaceError::MissingKeyword(
                "endcodespacerace".to_string(),
            ));
        }

        Ok(())
    }

    /// Try to read an integer at the current position.
    fn try_integer(&mut self) -> Result<i64, CodespaceError> {
        self.skip_whitespace_and_comments();
        let start = self.pos;

        // Check if we're at EOF before attempting to access input
        if self.pos >= self.input.len() {
            return Err(CodespaceError::UnexpectedToken(
                "expected integer".to_string(),
            ));
        }

        // Optional sign
        if self.pos < self.input.len()
            && (self.input[self.pos] == b'-' || self.input[self.pos] == b'+')
        {
            self.pos += 1;
        }

        // Digits
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }

        if self.pos == start {
            return Err(CodespaceError::UnexpectedToken(
                "expected integer".to_string(),
            ));
        }

        // Parse the integer
        let s = unsafe { std::str::from_utf8_unchecked(&self.input[start..self.pos]) };
        s.parse()
            .map_err(|_| CodespaceError::UnexpectedToken("invalid integer".to_string()))
    }

    /// Expect a hex string at the current position.
    fn expect_hex_string(&mut self) -> Result<Vec<u8>, CodespaceError> {
        self.skip_whitespace_and_comments();

        // Check if we're at EOF before attempting to access input
        if self.pos >= self.input.len() {
            return Err(CodespaceError::MissingKeyword("<hex string>".to_string()));
        }

        if self.input[self.pos] != b'<' {
            return Err(CodespaceError::UnexpectedToken("expected <".to_string()));
        }

        self.pos += 1;

        let mut result = Vec::new();
        let mut current_nibble: Option<u8> = None;

        while self.pos < self.input.len() {
            let b = self.input[self.pos];
            self.pos += 1;

            if b == b'>' {
                // End of hex string
                if let Some(hi) = current_nibble {
                    result.push(hi << 4);
                }
                return Ok(result);
            }

            // Try to parse hex digit
            if let Some(nibble) = Self::hex_digit_to_nibble(b) {
                if let Some(hi) = current_nibble {
                    result.push(hi << 4 | nibble);
                    current_nibble = None;
                } else {
                    current_nibble = Some(nibble);
                }
            } else if Self::is_whitespace(b) {
                // Whitespace is ignored
                continue;
            } else {
                return Err(CodespaceError::InvalidHexString(format!(
                    "invalid hex character: 0x{:02x}",
                    b
                )));
            }
        }

        // EOF before >
        if let Some(hi) = current_nibble {
            result.push(hi << 4);
        }
        Ok(result)
    }

    /// Try to match a keyword at the current position.
    fn try_keyword(&mut self, keyword: &[u8]) -> bool {
        self.skip_whitespace_and_comments();

        // Check if we're at EOF before attempting to match
        if self.pos >= self.input.len() {
            return false;
        }

        if self.input[self.pos..].starts_with(keyword) {
            // Check that the keyword is followed by whitespace or delimiter
            let next_pos = self.pos + keyword.len();
            if next_pos < self.input.len() {
                let next = self.input[next_pos];
                if !Self::is_whitespace(next) && !Self::is_delimiter(next) {
                    return false;
                }
            }
            self.pos += keyword.len();
            return true;
        }
        false
    }

    /// Skip whitespace and comments.
    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.input.len() {
            let b = self.input[self.pos];

            // Skip whitespace
            if Self::is_whitespace(b) {
                self.pos += 1;
                continue;
            }

            // Skip comment
            if b == b'%' {
                self.pos += 1;
                // Skip to end of line
                while self.pos < self.input.len() && self.input[self.pos] != b'\n' {
                    self.pos += 1;
                }
                // Include the newline
                if self.pos < self.input.len() {
                    self.pos += 1;
                }
                continue;
            }

            break;
        }
    }

    /// Skip whitespace only (not comments).
    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && Self::is_whitespace(self.input[self.pos]) {
            self.pos += 1;
        }
    }

    /// Skip a single token (until whitespace or delimiter).
    ///
    /// Advances position past the current token. A delimiter is consumed
    /// as a single-character token, whitespace stops before the token.
    fn skip_token(&mut self) {
        self.skip_whitespace_and_comments();
        while self.pos < self.input.len() {
            let b = self.input[self.pos];
            if Self::is_whitespace(b) {
                // Whitespace stops the token, don't consume it
                break;
            }
            // Consume this character
            self.pos += 1;
            if Self::is_delimiter(b) {
                // Delimiter stops after consuming it
                break;
            }
            // Regular character, keep consuming
        }
    }

    /// Skip tokens until we find the expected keyword.
    fn skip_to_keyword(&mut self, keyword: &[u8]) {
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10000; // Safety limit

        while self.pos < self.input.len() {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                panic!("CodespaceParser::skip_to_keyword exceeded maximum iterations ({}), likely infinite loop. Keyword: {:?}, Current position: {}, input length: {}",
                       MAX_ITERATIONS, std::str::from_utf8(keyword).unwrap_or("<invalid utf8>"), self.pos, self.input.len());
            }

            let pos_before = self.pos;

            self.skip_whitespace_and_comments();
            if self.try_keyword(keyword) {
                break;
            }
            self.skip_token();

            // Safety check: ensure we made progress; if not, advance by one to prevent infinite loop
            if self.pos == pos_before && self.pos < self.input.len() {
                eprintln!("Warning: skip_to_keyword made no progress, forcing position advance from {} to {}", self.pos, self.pos + 1);
                self.pos += 1;
            }

            // Safety: don't loop forever searching for a keyword that might not exist
            if self.pos >= self.input.len() {
                break;
            }
        }
    }

    /// Check if a byte is whitespace.
    fn is_whitespace(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0C')
    }

    /// Check if a byte is a delimiter.
    fn is_delimiter(b: u8) -> bool {
        matches!(
            b,
            b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%' | b'(' | b')'
        )
    }

    /// Convert a hex digit character to its 4-bit value.
    fn hex_digit_to_nibble(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }

    /// Emit an error as a diagnostic.
    fn emit_error(&mut self, error: &CodespaceError) {
        self.diagnostics.push(Diagnostic::with_dynamic(
            DiagCode::FontInvalidCmap,
            self.pos as u64,
            error.to_string(),
        ));
    }
}

/// Parse codespace ranges from raw bytes.
///
/// Convenience function that creates a parser and returns just the ranges.
pub fn parse_codespace_ranges(input: &[u8]) -> CodespaceRanges {
    let parser = CodespaceParser::new(input);
    let (ranges, _diagnostics) = parser.parse();
    ranges
}

/// Parse codespace ranges from raw bytes with diagnostics.
///
/// Returns both the ranges and any diagnostics generated during parsing.
pub fn parse_codespace_ranges_with_diags(input: &[u8]) -> (CodespaceRanges, Vec<Diagnostic>) {
    let parser = CodespaceParser::new(input);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_range_one_byte() {
        // Acceptance criterion: Parse <00> <7F> → 1 range, width=1
        let input = b"1 begincodespacerange\n<00> <7F>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, diags) = parser.parse();

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.ranges[0].width, 1);
        assert_eq!(ranges.ranges[0].lo_slice(), &[0x00]);
        assert_eq!(ranges.ranges[0].hi_slice(), &[0x7F]);
        assert!(diags.is_empty());
    }

    #[test]
    fn test_parse_two_ranges_mixed_width() {
        // Acceptance criterion: Parse <00> <7F> <8000> <FFFF> in one block
        let input = b"2 begincodespacerange\n<00> <7F>\n<8000> <FFFF>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, diags) = parser.parse();

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges.ranges[0].width, 1);
        assert_eq!(ranges.ranges[0].lo_slice(), &[0x00]);
        assert_eq!(ranges.ranges[0].hi_slice(), &[0x7F]);
        assert_eq!(ranges.ranges[1].width, 2);
        assert_eq!(ranges.ranges[1].lo_slice(), &[0x80, 0x00]);
        assert_eq!(ranges.ranges[1].hi_slice(), &[0xFF, 0xFF]);
        assert!(diags.is_empty());
    }

    #[test]
    fn test_width_inference() {
        // Acceptance criterion: 2-char hex → width=1; 4-char hex → width=2
        let input = b"2 begincodespacerange\n<AB> <CD>\n<1234> <5678>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, _) = parser.parse();

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges.ranges[0].width, 1);
        assert_eq!(ranges.ranges[1].width, 2);
    }

    #[test]
    fn test_case_insensitive_hex() {
        // Acceptance criterion: Case-insensitive hex (<C0> and <c0> equivalent)
        let input = b"1 begincodespacerange\n<C0> <c0>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, _) = parser.parse();

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.ranges[0].lo_slice(), &[0xC0]);
        assert_eq!(ranges.ranges[0].hi_slice(), &[0xC0]);
    }

    #[test]
    fn test_malformed_range_width_mismatch() {
        // Acceptance criterion: Width-mismatch lo/hi → diagnostic + skipped
        let input = b"1 begincodespacerange\n<00> <FFFF>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, diags) = parser.parse();

        // Range should be skipped
        assert_eq!(ranges.len(), 0);
        // Should emit diagnostic
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.code == DiagCode::FontInvalidCmap));
    }

    #[test]
    fn test_empty_cmap() {
        // Acceptance criterion: Empty CMap → empty ranges (defensive default applied elsewhere)
        let input = b"";
        let parser = CodespaceParser::new(input);
        let (ranges, _) = parser.parse();

        assert!(ranges.is_empty());
    }

    #[test]
    fn test_no_codespace_block() {
        // CMap with no begincodespacerange block
        let input = b"/CMapName /Identity-H def\n10 beginbfchar\n1 endbfchar";
        let parser = CodespaceParser::new(input);
        let (ranges, _) = parser.parse();

        assert!(ranges.is_empty());
    }

    #[test]
    fn test_hex_string_with_whitespace() {
        // Hex strings with internal whitespace should parse correctly
        let input = b"1 begincodespacerange\n<00 01> <7F>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, _) = parser.parse();

        // <00 01> with whitespace → 0x00 0x01 after parsing whitespace
        // Actually whitespace is ignored, so <00 01> becomes <0001> = 2 bytes
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.ranges[0].width, 2);
    }

    #[test]
    fn test_jis_range() {
        // JIS lead/trail 2-byte pattern
        let input = b"1 begincodespacerange\n<8140> <FEFE>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, _) = parser.parse();

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.ranges[0].width, 2);
        assert_eq!(ranges.ranges[0].lo_slice(), &[0x81, 0x40]);
        assert_eq!(ranges.ranges[0].hi_slice(), &[0xFE, 0xFE]);
    }

    #[test]
    fn test_three_byte_range() {
        // 3-byte range
        let input = b"1 begincodespacerange\n<800000> <FFFFFF>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, _) = parser.parse();

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.ranges[0].width, 3);
    }

    #[test]
    fn test_four_byte_range() {
        // 4-byte range
        let input = b"1 begincodespacerange\n<80000000> <FFFFFFFF>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, _) = parser.parse();

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.ranges[0].width, 4);
    }

    #[test]
    fn test_invalid_width_too_large() {
        // 5-byte range is invalid
        let input = b"1 begincodespacerange\n<0000000000> <FFFFFFFFFFFF>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, diags) = parser.parse();

        assert_eq!(ranges.len(), 0);
        assert!(!diags.is_empty());
    }

    #[test]
    fn test_find_range() {
        // Test finding a range for a given code
        let input = b"2 begincodespacerange\n<00> <7F>\n<8000> <FFFF>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, _) = parser.parse();

        // Single-byte code should match first range
        let range = ranges.find_range(&[0x41]);
        assert!(range.is_some());
        assert_eq!(range.unwrap().width, 1);

        // Two-byte code should match second range
        let range = ranges.find_range(&[0x81, 0x00]);
        assert!(range.is_some());
        assert_eq!(range.unwrap().width, 2);

        // Code outside all ranges should not match
        let range = ranges.find_range(&[0xFF, 0xFF, 0xFF]);
        assert!(range.is_none());
    }

    #[test]
    fn test_comment_in_block() {
        // Comments should be ignored
        let input = b"1 begincodespacerange\n% This is a comment\n<00> <7F>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, diags) = parser.parse();

        assert_eq!(ranges.len(), 1);
        assert!(diags.is_empty());
    }

    #[test]
    fn test_convenience_function() {
        // Test the convenience function
        let input = b"1 begincodespacerange\n<00> <7F>\nendcodespacerange";
        let ranges = parse_codespace_ranges(input);

        assert_eq!(ranges.len(), 1);
    }

    #[test]
    fn test_convenience_function_with_diags() {
        // Test the convenience function with diagnostics
        let input = b"1 begincodespacerange\n<00> <FFFF>\nendcodespacerange";
        let (ranges, diags) = parse_codespace_ranges_with_diags(input);

        assert_eq!(ranges.len(), 0);
        assert!(!diags.is_empty());
    }

    #[test]
    fn test_odd_length_hex_string() {
        // Odd-length hex string: <4> → 0x40 (dangling nibble padded)
        let input = b"1 begincodespacerange\n<4> <A>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, _) = parser.parse();

        assert_eq!(ranges.len(), 1);
        // <4> becomes 0x40, <A> becomes 0xA0
        assert_eq!(ranges.ranges[0].lo_slice(), &[0x40]);
        assert_eq!(ranges.ranges[0].hi_slice(), &[0xA0]);
    }

    #[test]
    fn test_recovery_on_error() {
        // Parse should continue after a malformed entry
        let input =
            b"3 begincodespacerange\n<00> <7F>\n<00> <FFFF>\n<8000> <FFFF>\nendcodespacerange";
        let parser = CodespaceParser::new(input);
        let (ranges, diags) = parser.parse();

        // First and third ranges should be parsed, second should be skipped
        assert_eq!(ranges.len(), 2);
        assert!(!diags.is_empty());
    }

    #[test]
    fn test_identity_h_roundtrip() {
        // Acceptance criterion: Round-trip with Identity-H CMap fixture
        // Identity-H CMap typically has a single 2-byte codespace range
        let identity_h_cmap = b"/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CMapName /Identity-H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 begincidchar
<0000> 0
endcidchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end";

        let parser = CodespaceParser::new(identity_h_cmap);
        let (ranges, diags) = parser.parse();

        // Identity-H should have a single 2-byte range covering all 16-bit codes
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.ranges[0].width, 2);
        assert_eq!(ranges.ranges[0].lo_slice(), &[0x00, 0x00]);
        assert_eq!(ranges.ranges[0].hi_slice(), &[0xFF, 0xFF]);
        assert!(diags.is_empty());

        // Verify that codes in this range are correctly identified
        let range = ranges.find_range(&[0x00, 0x41]).unwrap();
        assert_eq!(range.width, 2);
        let range = ranges.find_range(&[0xFF, 0xFF]).unwrap();
        assert_eq!(range.width, 2);
        let range = ranges.find_range(&[0x81, 0x40]).unwrap();
        assert_eq!(range.width, 2);
    }

    #[test]
    fn test_identity_v_roundtrip() {
        // Identity-V CMap similar to Identity-H but for vertical writing mode
        let identity_v_cmap = b"/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CMapName /Identity-V def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
endcmap
CMapName currentdict /CMap defineresource pop
end
end";

        let parser = CodespaceParser::new(identity_v_cmap);
        let (ranges, diags) = parser.parse();

        // Identity-V should have the same codespace as Identity-H
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.ranges[0].width, 2);
        assert!(diags.is_empty());
    }
}
