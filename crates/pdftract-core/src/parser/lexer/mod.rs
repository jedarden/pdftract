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
        // Invalidate peek cache on advancement
        self.peek_cache = None;

        // Skip whitespace and comments before dispatching
        self.skip_whitespace_and_comments();

        // Check for end of input
        if self.bytes.is_empty() {
            return Some(Token::Eof);
        }

        let _start_pos = self.pos;
        let token = self.lex_next();

        // If lexing returned None but we haven't reached EOF, something went wrong
        // Return Eof to signal end of parseable content
        if token.is_none() && !self.bytes.is_empty() {
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

        // Lex the next token
        let token = self.next_token();

        // Restore state
        self.pos = saved_pos;
        self.bytes = saved_bytes;

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

            // Skip until end of line
            while let Some(&b) = self.bytes.first() {
                self.advance(1);
                if b == b'\n' || b == b'\r' {
                    break;
                }
            }
        }
    }

    /// Internal: Skip whitespace and comments.
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            self.consume_whitespace();
            self.consume_comment();
            // If we consumed a comment, there might be more whitespace after it
            if !self.bytes.first().map_or(false, |&b| b == b'%') {
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

        // Handle leading sign
        if let Some(&b'-' | &b'+') = self.bytes.first() {
            self.advance(1);
        }

        // Parse digits and optional decimal point
        while let Some(&b) = self.bytes.first() {
            if b.is_ascii_digit() {
                has_digit = true;
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

        // Determine if integer or real
        if has_dot {
            // Real number - for now just return 0.0 as placeholder
            // Full implementation will parse the actual value
            Some(Token::Real(0.0))
        } else {
            // Integer - for now just return 0 as placeholder
            // Full implementation will parse the actual value
            Some(Token::Integer(0))
        }
    }

    fn lex_literal_string(&mut self) -> Option<Token> {
        // Placeholder - just consume to closing paren or EOF
        let start = self.pos;
        self.advance(1); // consume opening (
        let mut depth = 1;

        while let Some(&b) = self.bytes.first() {
            self.advance(1);
            match b {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(Token::String(Vec::new()));
                    }
                }
                b'\\' => {
                    // Skip escaped character
                    if let Some(_) = self.bytes.first() {
                        self.advance(1);
                    }
                }
                _ => {}
            }
        }

        // Unterminated string
        self.diagnostics.push(Diagnostic::with_static(
            DiagCode::UnterminatedString,
            start as u64,
            "Unterminated literal string",
        ));
        Some(Token::Null)
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
            self.advance(1);
            // Placeholder for hex string
            Some(Token::String(Vec::new()))
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
}
