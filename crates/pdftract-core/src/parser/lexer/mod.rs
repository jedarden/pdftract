//! PDF lexical analyzer (tokenizer).
//!
//! This module provides the lexer that converts raw PDF byte sequences into tokens.
//! PDF is byte-oriented; position tracking is byte-level, not character-level.

use crate::diagnostics::{Diagnostic as Diag, DiagCode};
use std::str::FromStr;

/// Token produced by the PDF lexer.
///
/// Each token represents a single lexical element from the PDF document.
/// String and Name tokens contain `Vec<u8>` because PDF names and strings
/// are byte sequences, not UTF-8 strings (encoding is determined later
/// by the font subsystem).
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    /// Boolean literal: `true` or `false`
    Bool(bool),
    /// Integer literal: e.g., `123`, `-7`
    Integer(i64),
    /// Real (floating-point) literal: e.g., `3.14`, `-.5`
    Real(f64),
    /// Literal string: e.g., `(Hello World)` with escape sequences
    String(Vec<u8>),
    /// Name object: e.g., `/Type`, `/Font#20File` (with hex escapes)
    Name(Vec<u8>),
    /// Array start delimiter: `[`
    ArrayStart,
    /// Array end delimiter: `]`
    ArrayEnd,
    /// Dictionary start delimiter: `<<`
    DictStart,
    /// Dictionary end delimiter: `>>`
    DictEnd,
    /// Stream keyword (followed by newline)
    Stream,
    /// End-stream keyword
    EndStream,
    /// Indirect object start: `obj`
    Obj,
    /// Indirect object end: `endobj`
    EndObj,
    /// Indirect reference: `R`
    IndirectRef,
    /// Null object: `null`
    Null,
    /// Keyword token for xref-resolver keywords and unknown keywords
    Keyword(Vec<u8>),
    /// End of input
    Eof,
}

/// PDF lexical analyzer.
///
/// The lexer processes PDF byte sequences and produces tokens.
/// It tracks byte position, accumulates diagnostics, and handles
/// whitespace and comments transparently.
///
/// # Example
///
/// ```ignore
/// let input = b"123 /Type (Hello)";
/// let mut lexer = Lexer::new(input);
///
/// assert_eq!(lexer.next_token(), Some(Token::Integer(123)));
/// assert_eq!(lexer.next_token(), Some(Token::Name(b"Type".to_vec())));
/// assert_eq!(lexer.next_token(), Some(Token::String(b"Hello".to_vec())));
/// assert_eq!(lexer.next_token(), Some(Token::Eof));
/// assert_eq!(lexer.next_token(), None);
/// ```
pub struct Lexer<'a> {
    /// Remaining input bytes
    bytes: &'a [u8],
    /// Current byte position within the original input
    pos: usize,
    /// Accumulated diagnostics
    diagnostics: Vec<Diag>,
    /// Cached token for peek operations (token, position after token)
    peek_cache: Option<(Token, usize)>,
    /// Whether Eof has been returned
    eof_returned: bool,
}

/// Lookup table for PDF whitespace characters.
///
/// PDF spec 7.2.2 defines whitespace as: NULL (0x00), HT (0x09), LF (0x0A),
/// FF (0x0C), CR (0x0D), and Space (0x20).
const WHITESPACE: [bool; 256] = {
    let mut table = [false; 256];
    table[0x00] = true; // NULL
    table[0x09] = true; // HT
    table[0x0A] = true; // LF
    table[0x0C] = true; // FF
    table[0x0D] = true; // CR
    table[0x20] = true; // Space
    table
};

/// Lookup table for PDF delimiter characters.
///
/// PDF spec 7.2.2 defines delimiters as: `(`, `)`, `<`, `>`, `[`, `]`, `{`, `}`, `/`, `%`.
const DELIMITERS: [bool; 256] = {
    let mut table = [false; 256];
    table[b'(' as usize] = true;
    table[b')' as usize] = true;
    table[b'<' as usize] = true;
    table[b'>' as usize] = true;
    table[b'[' as usize] = true;
    table[b']' as usize] = true;
    table[b'{' as usize] = true;
    table[b'}' as usize] = true;
    table[b'/' as usize] = true;
    table[b'%' as usize] = true;
    table
};

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given input bytes.
    ///
    /// # Example
    ///
    /// ```
    /// use pdftract_core::parser::lexer::Lexer;
    ///
    /// let lexer = Lexer::new(b"123 456");
    /// ```
    pub fn new(bytes: &'a [u8]) -> Self {
        Lexer {
            bytes,
            pos: 0,
            diagnostics: Vec::new(),
            peek_cache: None,
            eof_returned: false,
        }
    }

    /// Advance to the next token, returning it.
    ///
    /// Returns `Some(Token)` for each token in the input, ending with
    /// `Token::Eof`. After `Eof` is returned, subsequent calls return `None`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut lexer = Lexer::new(b"true false");
    /// assert_eq!(lexer.next_token(), Some(Token::Bool(true)));
    /// assert_eq!(lexer.next_token(), Some(Token::Bool(false)));
    /// ```
    pub fn next_token(&mut self) -> Option<Token> {
        // If Eof was already returned, return None
        if self.eof_returned {
            return None;
        }

        // Invalidate peek cache on advancement
        self.peek_cache = None;

        // Skip whitespace and comments before dispatching
        self.skip_whitespace_and_comments();

        // Check for end of input
        if self.bytes.is_empty() {
            self.eof_returned = true;
            return Some(Token::Eof);
        }

        let _start_pos = self.pos;
        let token = self.lex_next();

        // If lexing returned None but we haven't reached EOF, something went wrong
        // Return Eof to signal end of parseable content
        if token.is_none() {
            self.eof_returned = true;
            return Some(Token::Eof);
        }

        token
    }

    /// Peek at the next token without consuming it.
    ///
    /// Returns `Some(&Token)` for the next token, or `None` if at end of input.
    /// Consecutive peeks are cached and do not re-lex.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut lexer = Lexer::new(b"123");
    /// assert_eq!(lexer.peek_token(), Some(&Token::Integer(123)));
    /// assert_eq!(lexer.peek_token(), Some(&Token::Integer(123))); // Cached
    /// assert_eq!(lexer.next_token(), Some(Token::Integer(123)));
    /// ```
    pub fn peek_token(&mut self) -> Option<&Token> {
        // Use cache if available
        if self.peek_cache.is_some() {
            return self.peek_cache.as_ref().map(|(token, _)| token);
        }

        // Save current state
        let saved_pos = self.pos;
        let saved_bytes = self.bytes;
        let saved_eof_returned = self.eof_returned;

        // Lex the next token
        let token = self.next_token();

        // Restore state
        self.pos = saved_pos;
        self.bytes = saved_bytes;
        self.eof_returned = saved_eof_returned;

        // Cache the token if we got one
        if let Some(t) = token {
            self.peek_cache = Some((t.clone(), self.pos));
            // Return reference to the cached token
            return self.peek_cache.as_ref().map(|(token, _)| token);
        }

        None
    }

    /// Get the current byte position in the input.
    ///
    /// This returns the offset of the next byte to be consumed.
    /// Before calling `next_token()`, it points to the start of the next token.
    /// After calling `next_token()`, it points just past the consumed token.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut lexer = Lexer::new(b"123");
    /// assert_eq!(lexer.position(), 0);
    /// lexer.next_token();
    /// assert_eq!(lexer.position(), 3); // "123" is 3 bytes
    /// ```
    pub fn position(&self) -> u64 {
        self.pos as u64
    }

    /// Take all accumulated diagnostics, leaving the internal buffer empty.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut lexer = Lexer::new(b"% comment\n123");
    /// lexer.next_token();
    /// let diags = lexer.take_diagnostics();
    /// assert!(diags.is_empty());
    /// ```
    pub fn take_diagnostics(&mut self) -> Vec<Diag> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Peek at the token two positions ahead without consuming it.
    ///
    /// This is used for detecting indirect references (N G R pattern).
    /// Returns `Some(&Token)` for the second token ahead, or `None` if at end.
    pub fn peek2_token(&mut self) -> Option<Token> {
        // Save current state
        let saved_pos = self.pos;
        let saved_bytes = self.bytes;
        let saved_cache = self.peek_cache.take();
        let saved_eof_returned = self.eof_returned;

        // Consume first token
        let _first = self.next_token();

        // Peek at second token (clone it to avoid borrow issues)
        let second = self.peek_token().cloned();

        // Restore state
        self.pos = saved_pos;
        self.bytes = saved_bytes;
        self.peek_cache = saved_cache;
        self.eof_returned = saved_eof_returned;

        second
    }

    /// Skip n bytes in the input.
    ///
    /// This is used for recovery when we know how many bytes to skip.
    pub fn skip_bytes(&mut self, n: u64) -> usize {
        let to_skip = n.min(self.bytes.len() as u64) as usize;
        self.advance(to_skip);
        to_skip
    }

    /// Get the remaining bytes in the input.
    pub fn remaining_bytes(&self) -> &[u8] {
        self.bytes
    }

    /// Internal: Dispatch to the appropriate lexer based on the next byte.
    fn lex_next(&mut self) -> Option<Token> {
        let next = self.bytes.first()?;

        match next {
            b't' => self.lex_t_keyword(),
            b'f' => self.lex_f_keyword(),
            b'0'..=b'9' | b'-' | b'+' | b'.' => self.lex_numeric(),
            b'(' => self.lex_literal_string(),
            b'/' => self.lex_name(),
            b'[' => self.consume_and_return(Token::ArrayStart),
            b']' => self.consume_and_return(Token::ArrayEnd),
            b'<' => self.lex_angle_bracket(),
            b'>' => self.lex_right_angle(),
            b's' => self.lex_s_keyword(),
            b'e' => self.lex_e_keyword(),
            b'o' => self.lex_o_keyword(),
            b'R' => self.lex_r_keyword(),
            b'n' => self.lex_n_keyword(),
            b'x' => self.lex_x_keyword(),
            b'%' => self.lex_percent(),
            b'{' | b'}' | b')' => {
                // PDF 1.2 reserved {} for future use; ) outside string context is unexpected
                let pos = self.pos;
                self.diagnostics.push(Diag::with_dynamic(
                    DiagCode::StructUnexpectedByte,
                    pos as u64,
                    format!("Unexpected byte: 0x{:02x}", next),
                ));
                self.advance(1);
                Some(Token::Null)
            }
            _ => self.lex_keyword(),
        }
    }

    /// Internal: Consume one byte and return a token.
    fn consume_and_return(&mut self, token: Token) -> Option<Token> {
        self.advance(1);
        Some(token)
    }

    /// Internal: Advance by n bytes, updating position and bytes slice.
    fn advance(&mut self, n: usize) {
        self.bytes = self.bytes.get(n..).unwrap_or(&[]);
        self.pos += n;
    }

    /// Internal: Check if a byte is PDF whitespace.
    fn is_pdf_whitespace(b: u8) -> bool {
        WHITESPACE[b as usize]
    }

    /// Internal: Check if a byte is a PDF delimiter.
    fn is_pdf_delimiter(b: u8) -> bool {
        DELIMITERS[b as usize]
    }

    /// Internal: Skip whitespace characters.
    fn consume_whitespace(&mut self) {
        while let Some(&b) = self.bytes.first() {
            if Self::is_pdf_whitespace(b) {
                self.advance(1);
            } else {
                break;
            }
        }
    }

    /// Internal: Skip a comment (`%` to end of line).
    fn consume_comment(&mut self) {
        if let Some(&b'%') = self.bytes.first() {
            // Skip the %
            self.advance(1);

            // Skip until end of line (including the line ending character)
            while let Some(&b) = self.bytes.first() {
                self.advance(1);
                if b == b'\n' {
                    break;
                }
                if b == b'\r' {
                    // Also consume following \n if present (CRLF)
                    if let Some(&b'\n') = self.bytes.first() {
                        self.advance(1);
                    }
                    break;
                }
            }
        }
    }

    /// Internal: Skip whitespace and comments.
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            let had_whitespace = self.bytes.first().map_or(false, |&b| Self::is_pdf_whitespace(b));
            let had_comment = self.bytes.first() == Some(&b'%');

            self.consume_whitespace();
            self.consume_comment();

            // Continue looping if we had whitespace or a comment, and there's more input
            if !had_whitespace && !had_comment {
                break;
            }
            // If we consumed a comment, there might be more whitespace after it
            // If we consumed whitespace, there might be a comment after it
            if self.bytes.first().map_or(true, |&b| !Self::is_pdf_whitespace(b) && b != b'%') {
                break;
            }
        }
    }

    /// Stub implementations for token-specific lexers.
    /// These will be implemented in subsequent beads.

    fn lex_t_keyword(&mut self) -> Option<Token> {
        // Check for "true"
        if self.bytes.starts_with(b"true") {
            let next_after = self.bytes.get(4);
            if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
                self.advance(4);
                return Some(Token::Bool(true));
            }
        }
        // Check for "trailer"
        if self.bytes.starts_with(b"trailer") {
            let next_after = self.bytes.get(7);
            if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
                self.advance(7);
                return Some(Token::Keyword(b"trailer".to_vec()));
            }
        }
        // Not "true" or "trailer", treat as keyword
        self.lex_keyword()
    }

    fn lex_f_keyword(&mut self) -> Option<Token> {
        // Check for "false"
        if self.bytes.starts_with(b"false") {
            let next_after = self.bytes.get(5);
            if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
                self.advance(5);
                return Some(Token::Bool(false));
            }
        }
        // Not "false", treat as keyword
        self.lex_keyword()
    }

    fn lex_x_keyword(&mut self) -> Option<Token> {
        // Check for "xref"
        if self.bytes.starts_with(b"xref") {
            let next_after = self.bytes.get(4);
            if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
                self.advance(4);
                return Some(Token::Keyword(b"xref".to_vec()));
            }
        }
        // Not "xref", treat as keyword
        self.lex_keyword()
    }

    fn lex_percent(&mut self) -> Option<Token> {
        // Check for "%%EOF" - the PDF end-of-file marker
        if self.bytes.starts_with(b"%%EOF") {
            let next_after = self.bytes.get(5);
            if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
                self.advance(5);
                return Some(Token::Keyword(b"%%EOF".to_vec()));
            }
        }
        // Not "%%EOF", skip as a regular comment
        self.consume_comment();
        // After skipping comment, recurse to get next token
        self.skip_whitespace_and_comments();
        if self.bytes.is_empty() {
            self.eof_returned = true;
            return Some(Token::Eof);
        }
        self.lex_next()
    }

    fn lex_keyword(&mut self) -> Option<Token> {
        // Consume bytes until we hit a delimiter or whitespace
        let mut keyword_bytes = Vec::with_capacity(16);

        while let Some(&b) = self.bytes.first() {
            if Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b) {
                break;
            }
            keyword_bytes.push(b);
            self.advance(1);
        }

        if keyword_bytes.is_empty() {
            return Some(Token::Null);
        }

        // Unknown keywords emit Token::Keyword without a diagnostic
        // The object parser will validate against known operators and emit STRUCT_UNKNOWN_KEYWORD if needed
        Some(Token::Keyword(keyword_bytes))
    }

    fn lex_numeric(&mut self) -> Option<Token> {
        let start = self.pos;
        let input = self.bytes;

        // Track the number of sign characters and dots
        let mut sign_count = 0;
        let mut dot_count = 0;
        let mut has_digit = false;

        // First pass: consume the numeric prefix greedily
        let mut consumed = 0;

        // Consume optional leading sign (max one)
        if let Some(&b'-' | &b'+') = input.first() {
            sign_count = 1;
            consumed += 1;
        }

        // Consume digits before first dot (if any)
        while consumed < input.len() && input[consumed].is_ascii_digit() {
            has_digit = true;
            consumed += 1;
        }

        // Consume dots and following digits (loop to detect multiple dots)
        while consumed < input.len() && input[consumed] == b'.' {
            dot_count += 1;
            consumed += 1;

            // Consume digits after this dot (if any)
            while consumed < input.len() && input[consumed].is_ascii_digit() {
                has_digit = true;
                consumed += 1;
            }
        }

        // Validate: must have at least one digit
        if !has_digit {
            self.diagnostics.push(Diag::with_static(
                DiagCode::StructInvalidNumber,
                start as u64,
                "Numeric literal must contain at least one digit",
            ));
            // Consume what we scanned (at least the sign character)
            self.advance(consumed);
            return Some(Token::Integer(0));
        }

        // Validate: at most one dot
        if dot_count > 1 {
            self.diagnostics.push(Diag::with_static(
                DiagCode::StructInvalidNumber,
                start as u64,
                "Numeric literal may contain at most one decimal point",
            ));
            // Consume only the valid first part (up to second dot)
            let valid_end = consumed - (dot_count - 1); // Back up to before second dot
            self.advance(valid_end);
            return Some(Token::Integer(0));
        }

        // Check if we're at a boundary (whitespace or delimiter)
        // If not, we need to stop before the boundary character
        if consumed < input.len() {
            let next_byte = input[consumed];
            if !Self::is_pdf_whitespace(next_byte) && !Self::is_pdf_delimiter(next_byte) {
                // Check for scientific notation (e/E) - PDF doesn't support it
                // The 'e' or 'E' becomes the start of the next token
                if next_byte == b'e' || next_byte == b'E' {
                    // Stop before the 'e' - it's not part of the number
                } else {
                    // Some other non-delimiter character - stop here
                    // The consumed bytes are valid, so we proceed
                }
            }
        }

        // Extract the numeric literal as a string slice
        let num_bytes = &input[..consumed];

        // SAFETY: PDF numeric literals are ASCII-only per spec, so this is safe
        let num_str = unsafe { std::str::from_utf8_unchecked(num_bytes) };

        // Determine if integer or real
        if dot_count == 1 {
            // Real number - parse as f64
            match f64::from_str(num_str) {
                Ok(value) => {
                    self.advance(consumed);
                    Some(Token::Real(value))
                }
                Err(_) => {
                    // Parse failed - emit diagnostic and return 0.0
                    self.diagnostics.push(Diag::with_dynamic(
                        DiagCode::StructRealInvalid,
                        start as u64,
                        format!("Real number '{}' could not be parsed", num_str),
                    ));
                    self.advance(consumed);
                    Some(Token::Real(0.0))
                }
            }
        } else {
            // Integer - parse as i64
            match i64::from_str(num_str) {
                Ok(value) => {
                    self.advance(consumed);
                    Some(Token::Integer(value))
                }
                Err(_) => {
                    // Overflow - emit diagnostic and clamp to i64::MAX
                    self.diagnostics.push(Diag::with_dynamic(
                        DiagCode::StructIntegerOverflow,
                        start as u64,
                        format!("Integer '{}' exceeds i64 range, clamped to i64::MAX", num_str),
                    ));
                    self.advance(consumed);
                    Some(Token::Integer(i64::MAX))
                }
            }
        }
    }

    fn lex_literal_string(&mut self) -> Option<Token> {
        let start = self.pos;
        self.advance(1); // consume opening (
        let mut depth = 1;
        let mut result = Vec::with_capacity(64);

        while let Some(&b) = self.bytes.first() {
            match b {
                b'(' => {
                    self.advance(1);
                    depth += 1;
                    result.push(b'(');
                }
                b')' => {
                    self.advance(1);
                    depth -= 1;
                    if depth == 0 {
                        return Some(Token::String(result));
                    }
                    result.push(b')');
                }
                b'\\' => {
                    self.advance(1); // consume backslash
                    match self.bytes.first() {
                        Some(&b'n') => {
                            self.advance(1);
                            result.push(b'\n');
                        }
                        Some(&b'r') => {
                            self.advance(1);
                            result.push(b'\r');
                        }
                        Some(&b't') => {
                            self.advance(1);
                            result.push(b'\t');
                        }
                        Some(&b'b') => {
                            self.advance(1);
                            result.push(0x08);
                        }
                        Some(&b'f') => {
                            self.advance(1);
                            result.push(0x0C);
                        }
                        Some(&b'\\') => {
                            self.advance(1);
                            result.push(b'\\');
                        }
                        Some(&b'(') => {
                            self.advance(1);
                            depth += 1;
                            result.push(b'(');
                        }
                        Some(&b')') => {
                            self.advance(1);
                            // Emit literal ) without decreasing depth
                            result.push(b')');
                        }
                        Some(&b'\n') => {
                            // Line continuation: consume the \n, emit nothing
                            self.advance(1);
                        }
                        Some(&b'\r') => {
                            self.advance(1);
                            // Check for \r\n sequence
                            if let Some(&b'\n') = self.bytes.first() {
                                self.advance(1);
                            }
                            // Line continuation: emit nothing
                        }
                        Some(&d @ b'0'..=b'7') => {
                            // Octal escape: consume 1-3 octal digits
                            let mut value = (d - b'0') as u32;
                            self.advance(1);
                            let mut count = 1;

                            while count < 3 {
                                if let Some(&d @ b'0'..=b'7') = self.bytes.first() {
                                    value = value * 8 + (d - b'0') as u32;
                                    self.advance(1);
                                    count += 1;
                                } else {
                                    break;
                                }
                            }

                            if value > 255 {
                                self.diagnostics.push(Diag::with_dynamic(
                                    DiagCode::StructInvalidOctal,
                                    self.pos as u64,
                                    format!("Octal escape \\{:03o} exceeds 255, truncated", value),
                                ));
                                result.push((value & 0xFF) as u8);
                            } else {
                                result.push(value as u8);
                            }
                        }
                        Some(&other) => {
                            // Unknown escape: emit the character literally per PDF spec
                            self.advance(1);
                            result.push(other);
                        }
                        None => {
                            // Backslash at EOF - emit nothing and continue
                        }
                    }
                }
                _ => {
                    self.advance(1);
                    result.push(b);
                }
            }
        }

        // Unterminated string
        self.diagnostics.push(Diag::with_static(
            DiagCode::StructUnterminatedString,
            start as u64,
            "Unterminated literal string",
        ));
        Some(Token::String(result))
    }

    fn lex_name(&mut self) -> Option<Token> {
        let start = self.pos;
        self.advance(1); // consume the leading /

        let mut out = Vec::with_capacity(64);
        let mut raw_consumed: usize = 0;
        const MAX_RAW_BYTES: usize = 127;
        let mut truncated_due_to_length = false;

        // Loop until whitespace, delimiter, or length limit
        while raw_consumed < MAX_RAW_BYTES {
            let Some(&b) = self.bytes.first() else {
                break;
            };

            // Special check for NUL byte: it's whitespace per spec, but invalid in names
            if b == 0x00 {
                self.diagnostics.push(Diag::with_static(
                    DiagCode::StructInvalidName,
                    self.pos as u64,
                    "NUL byte in name is invalid per PDF spec",
                ));
                break;
            }

            // Check for termination: whitespace or delimiter
            if Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b) {
                break;
            }

            // Handle #XX hex escape
            if b == b'#' {
                // Check if adding a hex escape (3 raw bytes) would exceed the limit
                if raw_consumed + 3 > MAX_RAW_BYTES {
                    // Truncate before the # to avoid a half-decoded escape
                    truncated_due_to_length = true;
                    break;
                }

                // Need at least 2 more bytes for hex digits
                if self.bytes.len() >= 3 {
                    let hi = self.bytes[1];
                    let lo = self.bytes[2];

                    match (Self::hex_digit_to_nibble(hi), Self::hex_digit_to_nibble(lo)) {
                        (Some(h), Some(l)) => {
                            // Valid hex escape: decode to single byte
                            let decoded = (h << 4) | l;
                            // Check if decoded byte is NUL
                            if decoded == 0 {
                                self.diagnostics.push(Diag::with_static(
                                    DiagCode::StructInvalidName,
                                    self.pos as u64,
                                    "NUL byte in name is invalid per PDF spec",
                                ));
                                self.advance(3); // consume the #XX
                                break;
                            }
                            out.push(decoded);
                            self.advance(3);
                            raw_consumed += 3;
                        }
                        _ => {
                            // Invalid hex: emit diagnostic and treat # as literal
                            self.diagnostics.push(Diag::with_static(
                                DiagCode::StructInvalidName,
                                self.pos as u64,
                                "Invalid hex escape sequence in name",
                            ));
                            out.push(b'#');
                            self.advance(1);
                            raw_consumed += 1;
                        }
                    }
                } else {
                    // EOF before complete hex escape: treat # as literal
                    out.push(b'#');
                    self.advance(1);
                    raw_consumed += 1;
                }
            } else {
                // Regular byte: push as-is
                out.push(b);
                self.advance(1);
                raw_consumed += 1;
            }
        }

        // Emit diagnostic if we hit the length limit
        if truncated_due_to_length || raw_consumed > MAX_RAW_BYTES {
            self.diagnostics.push(Diag::with_static(
                DiagCode::StructInvalidName,
                start as u64,
                "Name exceeds 127-byte length limit",
            ));
        } else if raw_consumed == MAX_RAW_BYTES {
            // Check if there's more input that we didn't consume
            if let Some(&b) = self.bytes.first() {
                if !Self::is_pdf_whitespace(b) && !Self::is_pdf_delimiter(b) {
                    self.diagnostics.push(Diag::with_static(
                        DiagCode::StructInvalidName,
                        start as u64,
                        "Name exceeds 127-byte length limit",
                    ));
                }
            }
        }

        Some(Token::Name(out))
    }

    fn lex_angle_bracket(&mut self) -> Option<Token> {
        // Check for << (dict start) or < (hex string start)
        if self.bytes.len() >= 2 && self.bytes[1] == b'<' {
            self.advance(2);
            Some(Token::DictStart)
        } else {
            self.lex_hex_string()
        }
    }

    /// Parse a hex string of the form `<...>`.
    ///
    /// Hex strings contain pairs of hex digits that are decoded into bytes.
    /// Whitespace is ignored between hex digit pairs.
    /// If an odd number of hex digits is present, the final unpaired nibble
    /// is treated as the HIGH nibble of a final byte with LOW nibble 0.
    /// Example: `<4>` -> `\x40` (NOT `\x04`).
    fn lex_hex_string(&mut self) -> Option<Token> {
        let start = self.pos;
        self.advance(1); // consume opening <

        let mut out = Vec::with_capacity(32);
        let mut current_nibble: Option<u8> = None;

        while let Some(&b) = self.bytes.first() {
            if b == b'>' {
                // Terminating >
                self.advance(1);
                // If we have a dangling nibble, pad with low nibble 0
                if let Some(hi) = current_nibble {
                    out.push(hi << 4);
                }
                return Some(Token::String(out));
            }

            // Check for hex digit
            if let Some(nibble) = Self::hex_digit_to_nibble(b) {
                if let Some(hi) = current_nibble {
                    out.push(hi << 4 | nibble);
                    current_nibble = None;
                } else {
                    current_nibble = Some(nibble);
                }
                self.advance(1);
            } else if Self::is_pdf_whitespace(b) {
                // Whitespace is ignored
                self.advance(1);
            } else {
                // Invalid character - flush dangling nibble if present
                if let Some(hi) = current_nibble {
                    out.push(hi << 4);
                    current_nibble = None;
                }
                self.diagnostics.push(Diag::with_dynamic(
                    DiagCode::StructInvalidHex,
                    self.pos as u64,
                    format!("Invalid hex character '{}' (0x{:02x})", b as char, b),
                ));
                self.advance(1);
            }
        }

        // EOF before >
        self.diagnostics.push(Diag::with_static(
            DiagCode::StructUnterminatedString,
            start as u64,
            "Unterminated hex string",
        ));
        // Pad dangling nibble if present
        if let Some(hi) = current_nibble {
            out.push(hi << 4);
        }
        Some(Token::String(out))
    }

    /// Convert a hex digit character to its 4-bit value (0-15).
    /// Returns None if the character is not a valid hex digit.
    fn hex_digit_to_nibble(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }

    fn lex_right_angle(&mut self) -> Option<Token> {
        // Check for >> (dict end) or stray >
        if self.bytes.len() >= 2 && self.bytes[1] == b'>' {
            self.advance(2);
            Some(Token::DictEnd)
        } else {
            // Stray > - emit diagnostic
            self.diagnostics.push(Diag::with_static(
                DiagCode::StructUnexpectedByte,
                self.pos as u64,
                "Unexpected > character",
            ));
            self.advance(1);
            Some(Token::Null)
        }
    }

    fn lex_s_keyword(&mut self) -> Option<Token> {
        // Check for "stream"
        if self.bytes.starts_with(b"stream") {
            let next_after = self.bytes.get(6);
            if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
                self.advance(6);
                // Validate stream header: must be followed by \n or \r\n
                // PDF spec 7.3.8.1: stream keyword must be followed by \n or \r\n
                // A lone \r is INVALID
                let start_pos = self.pos;
                if let Some(&b'\n') = self.bytes.first() {
                    // \n is valid
                    self.advance(1); // consume the \n
                } else if let Some(&b'\r') = self.bytes.first() {
                    // \r\n is valid, lone \r is invalid
                    self.advance(1); // consume the \r
                    if let Some(&b'\n') = self.bytes.first() {
                        self.advance(1); // consume the \n
                    } else {
                        // Lone \r - invalid
                        self.diagnostics.push(Diag::with_static(
                            DiagCode::StructInvalidStreamHeader,
                            start_pos as u64,
                            "stream keyword must be followed by \\n or \\r\\n, not lone \\r",
                        ));
                    }
                } else {
                    // No line ending at all - invalid
                    self.diagnostics.push(Diag::with_static(
                        DiagCode::StructInvalidStreamHeader,
                        start_pos as u64,
                        "stream keyword must be followed by \\n or \\r\\n",
                    ));
                }

                return Some(Token::Stream);
            }
        }
        // Check for "startxref"
        if self.bytes.starts_with(b"startxref") {
            let next_after = self.bytes.get(10);
            if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
                self.advance(10);
                return Some(Token::Keyword(b"startxref".to_vec()));
            }
        }
        // Not "stream" or "startxref", treat as keyword or name
        self.lex_keyword()
    }

    fn lex_e_keyword(&mut self) -> Option<Token> {
        // Check for "endstream"
        if self.bytes.starts_with(b"endstream") {
            let next_after = self.bytes.get(9);
            if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
                self.advance(9);
                return Some(Token::EndStream);
            }
        }
        // Check for "endobj"
        if self.bytes.starts_with(b"endobj") {
            let next_after = self.bytes.get(7);
            if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
                self.advance(7);
                return Some(Token::EndObj);
            }
        }
        // Not a recognized keyword, treat as generic keyword
        self.lex_keyword()
    }

    fn lex_o_keyword(&mut self) -> Option<Token> {
        // Check for "obj"
        if self.bytes.starts_with(b"obj") {
            let next_after = self.bytes.get(3);
            if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
                self.advance(3);
                return Some(Token::Obj);
            }
        }
        // Not "obj", treat as generic keyword
        self.lex_keyword()
    }

    fn lex_r_keyword(&mut self) -> Option<Token> {
        // Check for "R" (indirect reference)
        let next_after = self.bytes.get(1);
        if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
            self.advance(1);
            Some(Token::IndirectRef)
        } else {
            self.lex_keyword()
        }
    }

    fn lex_n_keyword(&mut self) -> Option<Token> {
        // Check for "null"
        if self.bytes.starts_with(b"null") {
            let next_after = self.bytes.get(4);
            if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
                self.advance(4);
                return Some(Token::Null);
            }
        }
        // Not "null", treat as generic keyword
        self.lex_keyword()
    }

    fn lex_unknown(&mut self) -> Option<Token> {
        // Unknown character - skip it and emit diagnostic
        let pos = self.pos;
        self.diagnostics.push(Diag::with_dynamic(
            DiagCode::StructUnexpectedEof,
            pos as u64,
            format!("Unexpected byte: 0x{:02x}", self.bytes[0]),
        ));
        self.advance(1);
        Some(Token::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_eof_then_none() {
        let mut lexer = Lexer::new(b"");
        assert_eq!(lexer.next_token(), Some(Token::Eof));
        assert_eq!(lexer.next_token(), None);
    }

    #[test]
    fn whitespace_only_returns_eof() {
        let input = b"   \t\n\r%comment\n   ";
        let mut lexer = Lexer::new(input);
        assert_eq!(lexer.next_token(), Some(Token::Eof));
        assert_eq!(lexer.next_token(), None);
    }

    #[test]
    fn position_tracks_bytes() {
        let mut lexer = Lexer::new(b"123");
        assert_eq!(lexer.position(), 0);
        lexer.next_token();
        assert_eq!(lexer.position(), 3);
    }

    #[test]
    fn position_advances_through_whitespace() {
        let mut lexer = Lexer::new(b"   \t\n%comment\n   ");
        lexer.next_token();
        // Should advance through all whitespace and comment
        assert!(lexer.position() > 0);
    }

    #[test]
    fn bool_literals() {
        let mut lexer = Lexer::new(b"true false");
        assert_eq!(lexer.next_token(), Some(Token::Bool(true)));
        assert_eq!(lexer.next_token(), Some(Token::Bool(false)));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn bool_case_sensitive() {
        // "True" (capital T) is NOT the bool keyword - it's a generic keyword
        let mut lexer = Lexer::new(b"True");
        assert_eq!(lexer.next_token(), Some(Token::Keyword(b"True".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn array_delimiters() {
        let mut lexer = Lexer::new(b"[ ]");
        assert_eq!(lexer.next_token(), Some(Token::ArrayStart));
        assert_eq!(lexer.next_token(), Some(Token::ArrayEnd));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn dict_delimiters() {
        let mut lexer = Lexer::new(b"<< >>");
        assert_eq!(lexer.next_token(), Some(Token::DictStart));
        assert_eq!(lexer.next_token(), Some(Token::DictEnd));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn indirect_ref_keyword() {
        let mut lexer = Lexer::new(b"R");
        assert_eq!(lexer.next_token(), Some(Token::IndirectRef));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn null_keyword() {
        let mut lexer = Lexer::new(b"null");
        assert_eq!(lexer.next_token(), Some(Token::Null));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn obj_keywords() {
        let mut lexer = Lexer::new(b"obj endobj");
        assert_eq!(lexer.next_token(), Some(Token::Obj));
        assert_eq!(lexer.next_token(), Some(Token::EndObj));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn stream_keywords() {
        let mut lexer = Lexer::new(b"stream endstream");
        assert_eq!(lexer.next_token(), Some(Token::Stream));
        assert_eq!(lexer.next_token(), Some(Token::EndStream));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn stream_header_valid_line_endings() {
        // Test \n (valid)
        let mut lexer = Lexer::new(b"stream\nbody");
        assert_eq!(lexer.next_token(), Some(Token::Stream));
        let diags = lexer.take_diagnostics();
        assert!(diags.is_empty(), "No diagnostics for stream\\n");

        // Test \r\n (valid)
        let mut lexer = Lexer::new(b"stream\r\nbody");
        assert_eq!(lexer.next_token(), Some(Token::Stream));
        let diags = lexer.take_diagnostics();
        assert!(diags.is_empty(), "No diagnostics for stream\\r\\n");
    }

    #[test]
    fn stream_header_lone_cr_emits_diagnostic() {
        // Lone \r is invalid per PDF spec 7.3.8.1
        let mut lexer = Lexer::new(b"stream\rbody");
        assert_eq!(lexer.next_token(), Some(Token::Stream));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructInvalidStreamHeader);
        assert!(diags[0].message.as_ref().contains("lone \\r"));
    }

    #[test]
    fn stream_header_no_line_ending_emits_diagnostic() {
        // Stream keyword followed by space (not a line ending) is invalid
        let mut lexer = Lexer::new(b"stream body");
        assert_eq!(lexer.next_token(), Some(Token::Stream));
        let diags = lexer.take_diagnostics();
        assert!(!diags.is_empty(), "Should emit diagnostic for stream without proper line ending");
        assert!(diags.iter().any(|d| d.code == DiagCode::StructInvalidStreamHeader));
    }

    #[test]
    fn take_diagnostics_returns_empty_for_valid_input() {
        let mut lexer = Lexer::new(b"123");
        lexer.next_token();
        let diags = lexer.take_diagnostics();
        assert!(diags.is_empty());
    }

    #[test]
    fn take_diagnostics_clears_buffer() {
        let mut lexer = Lexer::new(b""); // Empty input won't produce diags, but we can test the API
        let diags1 = lexer.take_diagnostics();
        let diags2 = lexer.take_diagnostics();
        assert_eq!(diags1.len(), diags2.len());
    }

    // Literal string tests

    #[test]
    fn string_literal_balanced_parens() {
        let mut lexer = Lexer::new(b"(foo (bar) baz)");
        assert_eq!(
            lexer.next_token(),
            Some(Token::String(b"foo (bar) baz".to_vec()))
        );
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_empty() {
        let mut lexer = Lexer::new(b"()");
        assert_eq!(lexer.next_token(), Some(Token::String(b"".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_simple_text() {
        let mut lexer = Lexer::new(b"(Hello World)");
        assert_eq!(lexer.next_token(), Some(Token::String(b"Hello World".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_escape_newline() {
        let mut lexer = Lexer::new(b"(line1\\nline2)");
        assert_eq!(
            lexer.next_token(),
            Some(Token::String(b"line1\nline2".to_vec()))
        );
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_escape_carriage_return() {
        let mut lexer = Lexer::new(b"(line1\\rline2)");
        assert_eq!(
            lexer.next_token(),
            Some(Token::String(b"line1\rline2".to_vec()))
        );
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_escape_tab() {
        let mut lexer = Lexer::new(b"(col1\\tcol2)");
        assert_eq!(lexer.next_token(), Some(Token::String(b"col1\tcol2".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_escape_backspace() {
        let mut lexer = Lexer::new(b"(abc\\bdef)");
        assert_eq!(lexer.next_token(), Some(Token::String(b"abc\x08def".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_escape_form_feed() {
        let mut lexer = Lexer::new(b"(page1\\fpage2)");
        assert_eq!(
            lexer.next_token(),
            Some(Token::String(b"page1\x0Cpage2".to_vec()))
        );
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_escape_backslash() {
        let mut lexer = Lexer::new(b"(path\\\\file)");
        assert_eq!(lexer.next_token(), Some(Token::String(b"path\\file".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_escape_left_paren() {
        let mut lexer = Lexer::new(b"(\\(nested))");
        assert_eq!(lexer.next_token(), Some(Token::String(b"(nested)".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_escape_right_paren() {
        let mut lexer = Lexer::new(b"(\\)not_end)");
        assert_eq!(lexer.next_token(), Some(Token::String(b")not_end".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_octal_escape_single_digit() {
        let mut lexer = Lexer::new(b"(abc\\10)");
        assert_eq!(lexer.next_token(), Some(Token::String(b"abc\x08".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_octal_escape_two_digits() {
        let mut lexer = Lexer::new(b"(abc\\101)");
        assert_eq!(lexer.next_token(), Some(Token::String(b"abcA".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_octal_escape_three_digits() {
        let mut lexer = Lexer::new(b"(abc\\101\\102\\103)");
        assert_eq!(lexer.next_token(), Some(Token::String(b"abcABC".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_octal_escape_non_octal_following() {
        let mut lexer = Lexer::new(b"(abc\\10A)");
        assert_eq!(lexer.next_token(), Some(Token::String(b"abc\x08A".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_octal_escape_out_of_range_emits_diagnostic() {
        let mut lexer = Lexer::new(b"(abc\\401)");
        // Octal 401 = decimal 257, truncated to 1
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::String(b"abc\x01".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructInvalidOctal);
        assert!(diags[0].message.as_ref().contains("401"));
    }

    #[test]
    fn string_literal_line_continuation_lf() {
        let mut lexer = Lexer::new(b"(abc\\\ndef)");
        assert_eq!(lexer.next_token(), Some(Token::String(b"abcdef".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_line_continuation_cr() {
        let mut lexer = Lexer::new(b"(abc\\\rdef)");
        assert_eq!(lexer.next_token(), Some(Token::String(b"abcdef".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_line_continuation_crlf() {
        let mut lexer = Lexer::new(b"(abc\\\r\ndef)");
        assert_eq!(lexer.next_token(), Some(Token::String(b"abcdef".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_unknown_escape_emits_literal() {
        let mut lexer = Lexer::new(b"(abc\\qdef)");
        assert_eq!(lexer.next_token(), Some(Token::String(b"abcqdef".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn string_literal_unterminated_emits_diagnostic() {
        let mut lexer = Lexer::new(b"(unterminated");
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::String(b"unterminated".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructUnterminatedString);
    }

    #[test]
    fn string_literal_unterminated_with_escape() {
        let mut lexer = Lexer::new(b"(abc\\101");
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::String(b"abcA".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructUnterminatedString);
    }

    #[test]
    fn string_literal_deeply_nested_parens() {
        let mut lexer = Lexer::new(b"(((((x)))))");
        assert_eq!(
            lexer.next_token(),
            Some(Token::String(b"((((x))))".to_vec()))
        );
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }
    // Hex string tests

    #[test]
    fn hex_string_empty() {
        let mut lexer = Lexer::new(b"<>");
        assert_eq!(lexer.next_token(), Some(Token::String(b"".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn hex_string_odd_length_single_nibble() {
        let mut lexer = Lexer::new(b"<4>");
        // Critical test: <4> -> \x40 (NOT \x04)
        // The trailing zero nibble is LOW, not HIGH
        assert_eq!(lexer.next_token(), Some(Token::String(b"\x40".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn hex_string_hello_world() {
        let mut lexer = Lexer::new(b"<48656C6C6F>");
        // 48=H, 65=e, 6C=l, 6C=l, 6F=o
        assert_eq!(lexer.next_token(), Some(Token::String(b"Hello".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn hex_string_mixed_case() {
        let mut lexer = Lexer::new(b"<aBcD>");
        // aB=0xAB, cD=0xCD
        assert_eq!(lexer.next_token(), Some(Token::String(b"\xAB\xCD".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn hex_string_with_whitespace() {
        let mut lexer = Lexer::new(b"<48 65 6C\n6C 6F>");
        // Whitespace is ignored
        assert_eq!(lexer.next_token(), Some(Token::String(b"Hello".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn hex_string_odd_length_multiple_nibbles() {
        let mut lexer = Lexer::new(b"<48657>");
        // 48=0x48, 65=0x65, 7=0x70 (dangling nibble becomes HIGH nibble with LOW nibble 0)
        assert_eq!(lexer.next_token(), Some(Token::String(b"\x48\x65\x70".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn hex_string_invalid_char_emits_diagnostic() {
        let mut lexer = Lexer::new(b"<48Z65>");
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::String(b"\x48\x65".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructInvalidHex);
        // Debug: print actual message
        eprintln!("Actual diagnostic message: {}", diags[0].message.as_ref());
        assert!(diags[0].message.as_ref().contains("Z"));
    }

    #[test]
    fn hex_string_unterminated_emits_diagnostic() {
        let mut lexer = Lexer::new(b"<4865");
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::String(b"\x48\x65".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructUnterminatedString);
        assert!(diags[0].message.as_ref().contains("hex string"));
    }

    #[test]
    fn hex_string_unterminated_with_dangling_nibble() {
        let mut lexer = Lexer::new(b"<48657");
        // 48=0x48, 65=0x65, 7=0x70 (dangling nibble padded)
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::String(b"\x48\x65\x70".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructUnterminatedString);
    }

    #[test]
    fn hex_string_all_zero_bytes() {
        let mut lexer = Lexer::new(b"<000000>");
        assert_eq!(lexer.next_token(), Some(Token::String(b"\x00\x00\x00".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn hex_string_max_byte_value() {
        let mut lexer = Lexer::new(b"<FF>");
        assert_eq!(lexer.next_token(), Some(Token::String(b"\xFF".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn hex_string_lower_case_max_byte() {
        let mut lexer = Lexer::new(b"<ff>");
        assert_eq!(lexer.next_token(), Some(Token::String(b"\xFF".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn hex_string_multiple_invalid_chars() {
        let mut lexer = Lexer::new(b"<4X8Y>");
        let token = lexer.next_token();
        // X and Y are invalid, only 4 and 8 remain
        // 4 becomes 0x40, 8 becomes 0x80
        assert_eq!(token, Some(Token::String(b"\x40\x80".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 2);
        for diag in &diags {
            assert_eq!(diag.code, DiagCode::StructInvalidHex);
        }
    }

    #[test]
    fn hex_string_with_tab_whitespace() {
        let mut lexer = Lexer::new(b"<4\t8>");
        assert_eq!(lexer.next_token(), Some(Token::String(b"\x48".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn hex_string_dict_not_confused() {
        let mut lexer = Lexer::new(b"<<>>");
        // This is dict start/end, not a hex string
        assert_eq!(lexer.next_token(), Some(Token::DictStart));
        assert_eq!(lexer.next_token(), Some(Token::DictEnd));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn hex_string_vs_dict_start() {
        let mut lexer = Lexer::new(b"<<>");
        // << is dict start, > is stray
        assert_eq!(lexer.next_token(), Some(Token::DictStart));
        let token = lexer.next_token();
        // The stray > should produce a diagnostic
        assert!(matches!(token, Some(Token::Null)));
        let diags = lexer.take_diagnostics();
        assert!(!diags.is_empty());
    }

    #[test]
    fn hex_string_dict_start_hex_string_dict_end() {
        // Tricky case: <<<48>>> should be DictStart + String(b"\x48") + DictEnd
        // << = dict start, <48> = hex string, >> = dict end
        let mut lexer = Lexer::new(b"<<<48>>>");
        assert_eq!(lexer.next_token(), Some(Token::DictStart));
        assert_eq!(lexer.next_token(), Some(Token::String(b"\x48".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::DictEnd));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    // Proptests for hex string lexer

    #[test]
    fn proptest_hex_string_never_panics_on_random_bytes() {
        use proptest::prelude::*;

        // Generate random byte sequences that start with < (but not << to avoid dict start)
        let test_strategy = prop::collection::vec(prop::num::u8::ANY, 0..1000).prop_map(|mut bytes| {
            // Ensure the input starts with '<' but NOT '<<'
            // Insert '<' at the start, and ensure the second byte is not '<'
            bytes.insert(0, b'<');
            if bytes.len() > 1 && bytes[1] == b'<' {
                bytes[1] = b'>'; // Change second byte to something non-'<'
            }
            bytes
        });

        proptest!(|(bytes in test_strategy)| {
            // This should never panic
            let mut lexer = Lexer::new(&bytes);
            let _ = lexer.next_token();
        });
    }

    #[test]
    fn proptest_hex_string_roundtrip_via_reencode() {
        use proptest::prelude::*;

        // Helper function to encode bytes as a hex string
        fn encode_hex_string(bytes: &[u8]) -> Vec<u8> {
            let mut result = Vec::with_capacity(2 * bytes.len() + 2);
            result.push(b'<');
            for &b in bytes {
                result.push(hex_nibble_to_char((b >> 4) & 0x0F));
                result.push(hex_nibble_to_char(b & 0x0F));
            }
            result.push(b'>');
            result
        }

        fn hex_nibble_to_char(nibble: u8) -> u8 {
            match nibble {
                0..=9 => b'0' + nibble,
                10..=15 => b'a' + (nibble - 10),
                _ => b'0',
            }
        }

        // Generate valid hex strings and test roundtrip
        let test_strategy = prop::collection::vec(prop::num::u8::ANY, 0..100).prop_map(|bytes| {
            encode_hex_string(&bytes)
        });

        proptest!(|(encoded in test_strategy)| {
            let mut lexer = Lexer::new(&encoded);
            if let Some(Token::String(decoded)) = lexer.next_token() {
                // Re-encode the decoded bytes
                let reencoded = encode_hex_string(&decoded);

                // The re-encoded hex string should produce the same bytes when decoded again
                let mut lexer2 = Lexer::new(&reencoded);
                if let Some(Token::String(redecoded)) = lexer2.next_token() {
                    prop_assert_eq!(decoded, redecoded, "Roundtrip failed");
                } else {
                    prop_assert!(false, "Re-encoding did not produce a valid hex string");
                }
            } else {
                prop_assert!(false, "Encoded hex string did not decode to a String token");
            }
        });
    }

    // Proptests for string literal lexer

    #[test]
    fn proptest_string_never_panics_on_random_bytes() {
        use proptest::prelude::*;

        let test_strategy = prop::collection::vec(prop::num::u8::ANY, 0..1000).prop_map(|mut bytes| {
            // Ensure the input starts with '(' to trigger string lexing
            bytes.insert(0, b'(');
            bytes
        });

        proptest!(|(bytes in test_strategy)| {
            // This should never panic
            let mut lexer = Lexer::new(&bytes);
            let _ = lexer.next_token();
        });
    }

    #[test]
    fn proptest_valid_string_roundtrips() {
        use proptest::prelude::*;

        // Strategy for generating valid literal strings
        // We generate bytes that can appear in a PDF string and wrap them in parens
        let test_strategy = prop::collection::vec(
            prop::num::u8::ANY
                .prop_filter("avoid unprintable and special chars that make testing hard", |&b| {
                    // Allow most bytes, but filter out some that make roundtripping difficult
                    // We include parens but balance them manually
                    !matches!(b, 0x00 | 0x01..=0x08 | 0x0B | 0x0E..=0x1F)
                }),
            0..100,
        ).prop_map(|mut bytes| {
            // Balance parentheses: for every '(' we add a ')'
            let mut depth = 0i32;
            let mut result = Vec::new();
            result.push(b'(');
            for b in &bytes {
                if *b == b'(' {
                    depth += 1;
                } else if *b == b')' {
                    if depth > 0 {
                        depth -= 1;
                    } else {
                        // Skip unbalanced ')'
                        continue;
                    }
                }
                result.push(*b);
            }
            // Add closing parens to balance
            for _ in 0..depth {
                result.push(b')');
            }
            result.push(b')');
            result
        });

        proptest!(|(bytes in test_strategy)| {
            let mut lexer = Lexer::new(&bytes);
            if let Some(Token::String(s)) = lexer.next_token() {
                // A valid string should produce non-empty output
                // (unless the input was literally "()")
                if bytes.len() > 2 {
                    prop_assert!(!s.is_empty() || bytes == b"()");
                }
            } else {
                // Should always get a String token for well-formed input
                prop_assert!(false, "Expected String token, got {:?}", lexer.next_token());
            }
        });
    }

    // Name object tests

    #[test]
    fn name_simple() {
        let mut lexer = Lexer::new(b"/Foo");
        assert_eq!(lexer.next_token(), Some(Token::Name(b"Foo".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn name_with_hex_escape_space() {
        let mut lexer = Lexer::new(b"/Foo#20Bar");
        // #20 = 0x20 = space
        assert_eq!(lexer.next_token(), Some(Token::Name(b"Foo Bar".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn name_hex_escape_decodes_to_hash() {
        let mut lexer = Lexer::new(b"/#23#23");
        // #23 = 0x23 = #
        assert_eq!(lexer.next_token(), Some(Token::Name(b"##".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn name_empty() {
        let mut lexer = Lexer::new(b"/ ");
        // Empty name is valid per spec
        assert_eq!(lexer.next_token(), Some(Token::Name(b"".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn name_empty_followed_by_delimiter() {
        let mut lexer = Lexer::new(b"/[");
        // Empty name followed by [ delimiter (array start)
        assert_eq!(lexer.next_token(), Some(Token::Name(b"".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::ArrayStart));
    }

    #[test]
    fn name_nul_byte_rejected() {
        let mut lexer = Lexer::new(b"/Foo#00Bar");
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::Name(b"Foo".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructInvalidName);
        assert!(diags[0].message.as_ref().contains("NUL"));
    }

    #[test]
    fn name_literal_nul_byte_rejected() {
        let mut lexer = Lexer::new(b"/Foo\x00Bar");
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::Name(b"Foo".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructInvalidName);
    }

    #[test]
    fn name_length_limit_127_bytes() {
        // 128 A's - should truncate to 127 and emit diagnostic
        let mut input = vec![b'/'];
        input.extend(std::iter::repeat(b'A').take(128));
        let mut lexer = Lexer::new(&input);
        let token = lexer.next_token();
        if let Some(Token::Name(name)) = token {
            assert_eq!(name.len(), 127);
            assert!(name.iter().all(|&b| b == b'A'));
        } else {
            panic!("Expected Name token");
        }
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructInvalidName);
        assert!(diags[0].message.as_ref().contains("127"));
    }

    #[test]
    fn name_length_limit_exact_127_bytes_valid() {
        // Exactly 127 A's - should be valid
        let mut input = vec![b'/'];
        input.extend(std::iter::repeat(b'A').take(127));
        input.push(b' '); // delimiter
        let mut lexer = Lexer::new(&input);
        let token = lexer.next_token();
        if let Some(Token::Name(name)) = token {
            assert_eq!(name.len(), 127);
            assert!(name.iter().all(|&b| b == b'A'));
        } else {
            panic!("Expected Name token");
        }
        let diags = lexer.take_diagnostics();
        assert!(diags.is_empty(), "Expected no diagnostics for exactly 127 bytes");
    }

    #[test]
    fn name_length_limit_counts_raw_bytes_before_expansion() {
        // 124 A's + #41 (which expands to A) = 127 raw bytes, valid
        let mut input = vec![b'/'];
        input.extend(std::iter::repeat(b'A').take(124));
        input.extend_from_slice(b"#41");
        input.push(b' '); // delimiter
        let mut lexer = Lexer::new(&input);
        let token = lexer.next_token();
        if let Some(Token::Name(name)) = token {
            // 124 A's + 1 decoded A = 125 bytes
            assert_eq!(name.len(), 125);
            assert!(name.iter().all(|&b| b == b'A'));
        } else {
            panic!("Expected Name token");
        }
        let diags = lexer.take_diagnostics();
        assert!(diags.is_empty(), "Expected no diagnostics: 124 A's + #41 = 127 raw bytes");
    }

    #[test]
    fn name_truncation_before_incomplete_escape() {
        // 125 A's + # + 2 more chars = 128 raw bytes
        // Should truncate at 125, NOT include the #
        let mut input = vec![b'/'];
        input.extend(std::iter::repeat(b'A').take(125));
        input.push(b'#');
        input.push(b'4');
        input.push(b'1');
        let mut lexer = Lexer::new(&input);
        let token = lexer.next_token();
        if let Some(Token::Name(name)) = token {
            // Should be exactly 125 A's, truncated before the #
            assert_eq!(name.len(), 125);
            assert!(name.iter().all(|&b| b == b'A'));
        } else {
            panic!("Expected Name token");
        }
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructInvalidName);
    }

    #[test]
    fn name_invalid_hex_escape_keeps_hash_literal() {
        let mut lexer = Lexer::new(b"/Foo#GZ");
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::Name(b"Foo#GZ".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructInvalidName);
        assert!(diags[0].message.as_ref().contains("hex"));
    }

    #[test]
    fn name_invalid_hex_escape_single_digit() {
        let mut lexer = Lexer::new(b"/Foo#4");
        // EOF before second hex digit - treat # as literal
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::Name(b"Foo#4".to_vec())));
        // No diagnostic - this is a valid (if odd) name
        let diags = lexer.take_diagnostics();
        assert!(diags.is_empty());
    }

    #[test]
    fn name_hex_escape_mixed_case() {
        let mut lexer = Lexer::new(b"/#aB#cD#eF");
        // #aB = 0xAB, #cD = 0xCD, #eF = 0xEF
        assert_eq!(
            lexer.next_token(),
            Some(Token::Name(b"\xAB\xCD\xEF".to_vec()))
        );
    }

    #[test]
    fn name_case_sensitive() {
        let mut lexer = Lexer::new(b"/FooBar");
        assert_eq!(lexer.next_token(), Some(Token::Name(b"FooBar".to_vec())));
        // Names are case-sensitive - don't lowercase
        let mut lexer2 = Lexer::new(b"/foobar");
        assert_eq!(lexer2.next_token(), Some(Token::Name(b"foobar".to_vec())));
    }

    #[test]
    fn name_with_slash_delimiter() {
        let mut lexer = Lexer::new(b"/Foo/Bar");
        assert_eq!(lexer.next_token(), Some(Token::Name(b"Foo".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Name(b"Bar".to_vec())));
    }

    #[test]
    fn name_with_all_delimiters() {
        let mut lexer = Lexer::new(b"/Foo[Bar]");
        assert_eq!(lexer.next_token(), Some(Token::Name(b"Foo".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::ArrayStart));
        // Bar is not a name (doesn't start with /), so it's handled as a keyword
        // The object parser will reject unknown keywords
        assert_eq!(lexer.next_token(), Some(Token::Keyword(b"Bar".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::ArrayEnd));
    }

    #[test]
    fn name_with_bytes_preserved() {
        let mut lexer = Lexer::new(b"/\xFF\xFE\xFD");
        assert_eq!(
            lexer.next_token(),
            Some(Token::Name(b"\xFF\xFE\xFD".to_vec()))
        );
    }

    #[test]
    fn name_zero_byte_not_confused_with_nul() {
        let mut lexer = Lexer::new(b"/#30#30");
        // #30 = 0x30 = '0', not NUL
        assert_eq!(lexer.next_token(), Some(Token::Name(b"00".to_vec())));
        let diags = lexer.take_diagnostics();
        assert!(diags.is_empty());
    }

    #[test]
    fn name_hex_escape_zero_zero_is_nul() {
        let mut lexer = Lexer::new(b"/#00");
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::Name(b"".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructInvalidName);
    }

    #[test]
    fn name_multiple_invalid_hex_escapes() {
        let mut lexer = Lexer::new(b"/Foo#GZ#QX");
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::Name(b"Foo#GZ#QX".to_vec())));
        let diags = lexer.take_diagnostics();
        // Should have 2 diagnostics, one for each invalid escape
        assert_eq!(diags.len(), 2);
        for diag in &diags {
            assert_eq!(diag.code, DiagCode::StructInvalidName);
        }
    }

    #[test]
    fn name_proptest_never_panics_on_random_bytes() {
        use proptest::prelude::*;

        let test_strategy = prop::collection::vec(prop::num::u8::ANY, 0..1000).prop_map(|mut bytes| {
            // Ensure the input starts with '/' to trigger name lexing
            bytes.insert(0, b'/');
            bytes
        });

        proptest!(|(bytes in test_strategy)| {
            // This should never panic
            let mut lexer = Lexer::new(&bytes);
            let _ = lexer.next_token();
        });
    }

    #[test]
    fn name_proptest_always_produces_valid_token() {
        use proptest::prelude::*;

        let test_strategy = prop::collection::vec(prop::num::u8::ANY, 0..1000).prop_map(|mut bytes| {
            bytes.insert(0, b'/');
            bytes
        });

        proptest!(|(bytes in test_strategy)| {
            let mut lexer = Lexer::new(&bytes);
            if let Some(Token::Name(name)) = lexer.next_token() {
                // Name should never exceed 127 bytes (raw input length)
                // The decoded name may be shorter due to hex expansion
                prop_assert!(name.len() <= 127, "Name length {} exceeds 127", name.len());
            } else {
                // Should always get a Name token for input starting with /
                prop_assert!(false, "Expected Name token, got {:?}", lexer.next_token());
            }
        });
    }

    // Acceptance criteria tests for pdftract-5upi

    #[test]
    fn array_with_integers() {
        // Acceptance: [1 2 3] -> ArrayStart, Integer(1), Integer(2), Integer(3), ArrayEnd, Eof
        let mut lexer = Lexer::new(b"[1 2 3]");
        assert_eq!(lexer.next_token(), Some(Token::ArrayStart));
        assert_eq!(lexer.next_token(), Some(Token::Integer(1)));
        assert_eq!(lexer.next_token(), Some(Token::Integer(2)));
        assert_eq!(lexer.next_token(), Some(Token::Integer(3)));
        assert_eq!(lexer.next_token(), Some(Token::ArrayEnd));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn dict_with_name_and_integer() {
        // Acceptance: << /A 1 >> -> DictStart, Name(b"A"), Integer(1), DictEnd, Eof
        let mut lexer = Lexer::new(b"<< /A 1 >>");
        assert_eq!(lexer.next_token(), Some(Token::DictStart));
        assert_eq!(lexer.next_token(), Some(Token::Name(b"A".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Integer(1)));
        assert_eq!(lexer.next_token(), Some(Token::DictEnd));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn indirect_object_header_and_null() {
        // Acceptance: 12 0 obj null endobj -> Integer(12), Integer(0), Obj, Null, EndObj, Eof
        let mut lexer = Lexer::new(b"12 0 obj null endobj");
        assert_eq!(lexer.next_token(), Some(Token::Integer(12)));
        assert_eq!(lexer.next_token(), Some(Token::Integer(0)));
        assert_eq!(lexer.next_token(), Some(Token::Obj));
        assert_eq!(lexer.next_token(), Some(Token::Null));
        assert_eq!(lexer.next_token(), Some(Token::EndObj));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn indirect_reference_pattern() {
        // Acceptance: 5 0 R -> Integer(5), Integer(0), IndirectRef, Eof
        let mut lexer = Lexer::new(b"5 0 R");
        assert_eq!(lexer.next_token(), Some(Token::Integer(5)));
        assert_eq!(lexer.next_token(), Some(Token::Integer(0)));
        assert_eq!(lexer.next_token(), Some(Token::IndirectRef));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn bool_and_null_sequence() {
        // Acceptance: true false null -> Bool(true), Bool(false), Null, Eof
        let mut lexer = Lexer::new(b"true false null");
        assert_eq!(lexer.next_token(), Some(Token::Bool(true)));
        assert_eq!(lexer.next_token(), Some(Token::Bool(false)));
        assert_eq!(lexer.next_token(), Some(Token::Null));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    // Numeric literal tests for pdftract-1jjn

    #[test]
    fn numeric_integer_positive() {
        let mut lexer = Lexer::new(b"123");
        assert_eq!(lexer.next_token(), Some(Token::Integer(123)));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn numeric_integer_negative() {
        let mut lexer = Lexer::new(b"-7");
        assert_eq!(lexer.next_token(), Some(Token::Integer(-7)));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn numeric_real_simple() {
        let mut lexer = Lexer::new(b"3.14");
        assert_eq!(lexer.next_token(), Some(Token::Real(3.14)));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn numeric_real_negative_dot_then_digits() {
        // Acceptance: -.5 -> Real(-0.5)
        let mut lexer = Lexer::new(b"-.5");
        assert_eq!(lexer.next_token(), Some(Token::Real(-0.5)));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn numeric_real_digits_then_dot() {
        // Acceptance: 42. -> Real(42.0)
        let mut lexer = Lexer::new(b"42.");
        assert_eq!(lexer.next_token(), Some(Token::Real(42.0)));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn numeric_real_dot_then_digits() {
        // Acceptance: .001 -> Real(0.001)
        let mut lexer = Lexer::new(b".001");
        assert_eq!(lexer.next_token(), Some(Token::Real(0.001)));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn numeric_integer_positive_zero() {
        // Acceptance: +0 -> Integer(0)
        let mut lexer = Lexer::new(b"+0");
        assert_eq!(lexer.next_token(), Some(Token::Integer(0)));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn numeric_scientific_notation_rejected() {
        // Acceptance: 1e5 -> Integer(1) followed by Name(b"e5") or similar
        // PDF does NOT support scientific notation
        let mut lexer = Lexer::new(b"1e5");
        assert_eq!(lexer.next_token(), Some(Token::Integer(1)));
        // The 'e5' becomes a keyword (not a name since it doesn't start with /)
        assert_eq!(lexer.next_token(), Some(Token::Keyword(b"e5".to_vec())));
        assert_eq!(lexer.next_token(), Some(Token::Eof));
    }

    #[test]
    fn numeric_overflow_clamps_to_max() {
        // Acceptance: 99999999999999999999 (overflow) -> Integer(i64::MAX) with STRUCT_INTEGER_OVERFLOW
        let mut lexer = Lexer::new(b"99999999999999999999");
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::Integer(i64::MAX)));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::StructIntegerOverflow);
    }

    #[test]
    fn numeric_double_sign_emits_diagnostic() {
        // Acceptance: --5 -> diagnostic STRUCT_INVALID_NUMBER
        let mut lexer = Lexer::new(b"--5");
        let token = lexer.next_token();
        // Should emit diagnostic and return Integer(0) or similar
        assert!(matches!(token, Some(Token::Integer(0)) | Some(Token::Null)));
        let diags = lexer.take_diagnostics();
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.code == DiagCode::StructInvalidNumber));
    }

    #[test]
    fn numeric_real_negative_dot_then_digits_with_boundary() {
        // -.5 followed by delimiter
        let mut lexer = Lexer::new(b"-.5[");
        assert_eq!(lexer.next_token(), Some(Token::Real(-0.5)));
        assert_eq!(lexer.next_token(), Some(Token::ArrayStart));
    }

    #[test]
    fn numeric_multiple_dots_emits_diagnostic() {
        // 1.2.3 should emit STRUCT_INVALID_NUMBER
        let mut lexer = Lexer::new(b"1.2.3");
        let token = lexer.next_token();
        // Should consume up to second dot and emit diagnostic
        assert!(matches!(token, Some(Token::Integer(0)) | Some(Token::Real(_))));
        let diags = lexer.take_diagnostics();
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.code == DiagCode::StructInvalidNumber));
    }

    #[test]
    fn numeric_bare_sign_emits_diagnostic() {
        // A bare + or - with no following digits is invalid
        let mut lexer = Lexer::new(b"+");
        let token = lexer.next_token();
        assert!(matches!(token, Some(Token::Integer(0)) | Some(Token::Null)));
        let diags = lexer.take_diagnostics();
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.code == DiagCode::StructInvalidNumber));
    }

    #[test]
    fn numeric_hex_notation_not_supported() {
        // 0xFF is NOT a numeric literal in PDF
        // The 'x' terminates the number at position 1
        let mut lexer = Lexer::new(b"0xFF");
        assert_eq!(lexer.next_token(), Some(Token::Integer(0)));
        // The 'xFF' becomes a keyword
        assert_eq!(lexer.next_token(), Some(Token::Keyword(b"xFF".to_vec())));
    }

    #[test]
    fn proptest_numeric_never_panics() {
        use proptest::prelude::*;

        // Generate random byte sequences starting with numeric characters
        let test_strategy = prop::collection::vec(prop::num::u8::ANY, 0..1000).prop_map(|mut bytes| {
            // Ensure the input starts with a numeric-start character (+, -, ., 0-9)
            if bytes.is_empty() {
                bytes.push(b'1');
            } else {
                let numeric_starts = [b'+', b'-', b'.', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9'];
                bytes[0] = numeric_starts[bytes[0] as usize % numeric_starts.len()];
            }
            bytes
        });

        proptest!(|(bytes in test_strategy)| {
            // This should never panic
            let mut lexer = Lexer::new(&bytes);
            let _ = lexer.next_token();
        });
    }

    #[test]
    fn proptest_random_bytes_never_panics() {
        use proptest::prelude::*;

        // Generate random byte sequences and verify they never panic
        let test_strategy = prop::collection::vec(prop::num::u8::ANY, 0..1000);

        proptest!(|(bytes in test_strategy)| {
            // This should never panic
            let mut lexer = Lexer::new(&bytes);

            // Consume all tokens until Eof
            let mut token_count = 0;
            let max_tokens = 10000; // Safety limit to prevent infinite loops

            while token_count < max_tokens {
                match lexer.next_token() {
                    Some(Token::Eof) | None => break,
                    Some(_) => token_count += 1,
                }
            }

            // Should always terminate with Eof
            prop_assert!(token_count < max_tokens, "Token stream did not terminate");
        });
    }
}
