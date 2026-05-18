//! PDF lexical analyzer (tokenizer).
//!
//! This module provides the lexer that converts raw PDF byte sequences into tokens.
//! PDF is byte-oriented; position tracking is byte-level, not character-level.

use std::borrow::Cow;

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
    /// End of input
    Eof,
}

/// Diagnostic code for lexer errors.
///
/// All lexer diagnostic codes use the `STRUCT_` prefix to indicate
/// they relate to structural/lexical issues in the PDF document.
#[derive(Clone, Debug, PartialEq)]
pub enum DiagCode {
    /// Invalid name character or malformed name
    InvalidName,
    /// Invalid hexadecimal character in hex string or name escape
    InvalidHex,
    /// Invalid octal escape sequence in literal string
    InvalidOctal,
    /// Invalid stream header (stream keyword not followed by proper newline)
    InvalidStreamHeader,
    /// Unexpected end of file while parsing a token
    UnexpectedEof,
    /// Unterminated literal string (missing closing paren)
    UnterminatedString,
}

/// Diagnostic message emitted during lexing.
///
/// Diagnostics are accumulated during lexing and can be retrieved
/// via `Lexer::take_diagnostics()`. They do not stop lexing; the
/// lexer attempts recovery and continues.
///
/// Diagnostic messages use `Cow<'static, str>` so static error messages
/// don't allocate. Dynamic messages (with formatting) allocate only when needed.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    /// The diagnostic code identifying the type of error
    pub code: DiagCode,
    /// Byte offset in the input where the error occurred
    pub byte_offset: u64,
    /// Human-readable error message
    pub msg: Cow<'static, str>,
}

impl Diagnostic {
    /// Create a diagnostic with a static message (no allocation).
    fn with_static(code: DiagCode, byte_offset: u64, msg: &'static str) -> Self {
        Diagnostic {
            code,
            byte_offset,
            msg: Cow::Borrowed(msg),
        }
    }

    /// Create a diagnostic with a dynamic message (allocates).
    fn with_dynamic(code: DiagCode, byte_offset: u64, msg: String) -> Self {
        Diagnostic {
            code,
            byte_offset,
            msg: Cow::Owned(msg),
        }
    }
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
    diagnostics: Vec<Diagnostic>,
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
    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
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
            b't' | b'f' => self.lex_bool(),
            b'0'..=b'9' | b'-' | b'+' => self.lex_numeric(),
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
            _ => self.lex_unknown(),
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

    fn lex_bool(&mut self) -> Option<Token> {
        // Check for "true" or "false"
        if self.bytes.starts_with(b"true") {
            let next_after = self.bytes.get(4);
            if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
                self.advance(4);
                return Some(Token::Bool(true));
            }
        }
        if self.bytes.starts_with(b"false") {
            let next_after = self.bytes.get(5);
            if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
                self.advance(5);
                return Some(Token::Bool(false));
            }
        }
        // Not a bool, fall through to name lexing (e.g., "trueValue")
        self.lex_name()
    }

    fn lex_numeric(&mut self) -> Option<Token> {
        let start = self.pos;
        let mut has_dot = false;
        let mut has_digit = false;
        let mut value: i64 = 0;
        let mut sign: i64 = 1;

        // Handle leading sign
        if let Some(&b'-' | &b'+') = self.bytes.first() {
            if self.bytes.first() == Some(&b'-') {
                sign = -1;
            }
            self.advance(1);
        }

        // Parse digits and optional decimal point
        while let Some(&b) = self.bytes.first() {
            if b.is_ascii_digit() {
                has_digit = true;
                // Check for overflow
                if let Some(new_value) = value.checked_mul(10) {
                    if let Some(with_digit) = new_value.checked_add((b - b'0') as i64) {
                        value = with_digit;
                    } else {
                        // Overflow - clamp to max value
                        value = i64::MAX;
                    }
                } else {
                    // Overflow - clamp to max value
                    value = i64::MAX;
                }
                self.advance(1);
            } else if b == b'.' && !has_dot {
                has_dot = true;
                self.advance(1);
            } else {
                break;
            }
        }

        if !has_digit {
            // Not a valid number, emit diagnostic and return null
            self.diagnostics.push(Diagnostic::with_static(
                DiagCode::UnexpectedEof,
                start as u64,
                "Invalid numeric literal",
            ));
            return Some(Token::Null);
        }

        // Apply sign
        value = value * sign;

        // Determine if integer or real
        if has_dot {
            // Real number - parse as f64 by reconstructing the string
            // For now, just return the integer part as a real
            Some(Token::Real(value as f64))
        } else {
            // Integer
            Some(Token::Integer(value))
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
                                self.diagnostics.push(Diagnostic::with_dynamic(
                                    DiagCode::InvalidOctal,
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
        self.diagnostics.push(Diagnostic::with_static(
            DiagCode::UnterminatedString,
            start as u64,
            "Unterminated literal string",
        ));
        Some(Token::String(result))
    }

    fn lex_name(&mut self) -> Option<Token> {
        // Skip the /
        self.advance(1);

        // Consume name characters
        while let Some(&b) = self.bytes.first() {
            if Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b) {
                break;
            }
            self.advance(1);
        }

        Some(Token::Name(Vec::new()))
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
                self.diagnostics.push(Diagnostic::with_dynamic(
                    DiagCode::InvalidHex,
                    self.pos as u64,
                    format!("Invalid hex character '{}' (0x{:02x})", b as char, b),
                ));
                self.advance(1);
            }
        }

        // EOF before >
        self.diagnostics.push(Diagnostic::with_static(
            DiagCode::UnterminatedString,
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
            self.diagnostics.push(Diagnostic::with_static(
                DiagCode::UnexpectedEof,
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
                // Validate stream header (must be followed by \n or \r\n)
                // Placeholder for now
                return Some(Token::Stream);
            }
        }
        // Not "stream", treat as name
        self.lex_name()
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
        // Not a recognized keyword, treat as name
        self.lex_name()
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
        // Not "obj", treat as name
        self.lex_name()
    }

    fn lex_r_keyword(&mut self) -> Option<Token> {
        // Check for "R" (indirect reference)
        let next_after = self.bytes.get(1);
        if next_after.map_or(true, |&b| Self::is_pdf_whitespace(b) || Self::is_pdf_delimiter(b)) {
            self.advance(1);
            Some(Token::IndirectRef)
        } else {
            self.lex_name()
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
        // Not "null", treat as name
        self.lex_name()
    }

    fn lex_unknown(&mut self) -> Option<Token> {
        // Unknown character - skip it and emit diagnostic
        let pos = self.pos;
        self.diagnostics.push(Diagnostic::with_dynamic(
            DiagCode::UnexpectedEof,
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
        assert_eq!(diags[0].code, DiagCode::InvalidOctal);
        assert!(diags[0].msg.contains("401"));
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
        assert_eq!(diags[0].code, DiagCode::UnterminatedString);
    }

    #[test]
    fn string_literal_unterminated_with_escape() {
        let mut lexer = Lexer::new(b"(abc\\101");
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::String(b"abcA".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::UnterminatedString);
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
        assert_eq!(diags[0].code, DiagCode::InvalidHex);
        // Debug: print actual message
        eprintln!("Actual diagnostic message: {}", diags[0].msg);
        assert!(diags[0].msg.contains("Z"));
    }

    #[test]
    fn hex_string_unterminated_emits_diagnostic() {
        let mut lexer = Lexer::new(b"<4865");
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::String(b"\x48\x65".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::UnterminatedString);
        assert!(diags[0].msg.contains("hex string"));
    }

    #[test]
    fn hex_string_unterminated_with_dangling_nibble() {
        let mut lexer = Lexer::new(b"<48657");
        // 48=0x48, 65=0x65, 7=0x70 (dangling nibble padded)
        let token = lexer.next_token();
        assert_eq!(token, Some(Token::String(b"\x48\x65\x70".to_vec())));
        let diags = lexer.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::UnterminatedString);
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
            assert_eq!(diag.code, DiagCode::InvalidHex);
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
}
