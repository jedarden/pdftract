//! PDF object parser.
//!
//! This module provides the parser that converts tokens from the lexer
//! into PDF objects.

use super::types::{intern, ObjRef, PdfDict, PdfObject, PdfStream, PdfIndirect};
use crate::parser::lexer::{Lexer, Token};
use crate::diagnostics::{Diagnostic as Diag, DiagCode};

/// Maximum nesting depth for dictionaries and arrays.
///
/// Real PDFs rarely exceed 30 levels; this limit protects against
/// adversarial input that could cause stack overflow.
const MAX_DEPTH: u16 = 256;

/// PDF object parser.
///
/// Consumes tokens from the lexer and produces PDF objects.
/// Handles all direct object variants including nested structures.
pub struct ObjectParser<'a> {
    /// The lexer that provides tokens
    lexer: Lexer<'a>,
    /// Accumulated diagnostics
    diagnostics: Vec<Diag>,
    /// Current nesting depth (for depth limit enforcement)
    depth: u16,
}

impl<'a> ObjectParser<'a> {
    /// Create a new object parser.
    ///
    /// # Example
    ///
    /// ```
    /// use pdftract_core::parser::object::ObjectParser;
    ///
    /// let parser = ObjectParser::new(b"123");
    /// ```
    pub fn new(bytes: &'a [u8]) -> Self {
        ObjectParser {
            lexer: Lexer::new(bytes),
            diagnostics: Vec::new(),
            depth: 0,
        }
    }

    /// Get the current byte position in the input.
    pub fn position(&self) -> u64 {
        self.lexer.position()
    }

    /// Take all accumulated diagnostics.
    pub fn take_diagnostics(&mut self) -> Vec<Diag> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Parse the next direct object from the token stream.
    ///
    /// This method handles all PDF object variants:
    /// - Null, Bool, Integer, Real, String, Name
    /// - Array (recursive)
    /// - Dictionary (recursive)
    /// - Stream (dictionary followed by stream keyword)
    /// - Indirect reference (N G R pattern)
    ///
    /// Returns `None` on EOF.
    ///
    /// # Example
    ///
    /// ```
    /// use pdftract_core::parser::object::ObjectParser;
    ///
    /// let mut parser = ObjectParser::new(b"123");
    /// let obj = parser.parse_direct_object();
    /// assert!(obj.is_some());
    /// ```
    pub fn parse_direct_object(&mut self) -> Option<PdfObject> {
        let token = self.lexer.next_token()?;

        match token {
            Token::Null => Some(PdfObject::Null),
            Token::Bool(b) => Some(PdfObject::Bool(b)),
            Token::Integer(i) => self.parse_integer_or_ref(i),
            Token::Real(r) => Some(PdfObject::Real(r)),
            Token::String(s) => Some(PdfObject::String(Box::new(s))),
            Token::Name(n) => {
                // Convert bytes to string, lossily replacing invalid UTF-8
                let s = String::from_utf8_lossy(&n);
                Some(PdfObject::Name(intern(&s)))
            }
            Token::ArrayStart => self.parse_array(),
            Token::DictStart => self.parse_dict(),
            Token::Eof => None,
            _ => {
                // Unexpected token - emit diagnostic and return null
                self.diagnostics.push(Diag::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedByte,
                    format!("Unexpected token: {:?}", token),
                ));
                Some(PdfObject::Null)
            }
        }
    }

    /// Parse an integer or an indirect reference.
    ///
    /// Indirect references have the pattern: `Integer Integer R`
    /// We need 2-token lookahead to detect this.
    fn parse_integer_or_ref(&mut self, first_int: i64) -> Option<PdfObject> {
        // Peek ahead to see if this is an indirect reference
        let peek1 = self.lexer.peek_token().map(|t| t.clone());
        let peek2 = self.lexer.peek2_token();

        if let (Some(Token::Integer(gen)), Some(Token::IndirectRef)) = (peek1, peek2) {
            // This is an indirect reference: N G R
            // Consume the generation number and R
            let _ = self.lexer.next_token(); // Integer (gen)
            let _ = self.lexer.next_token(); // IndirectRef (R)

            // Validate object and generation numbers are non-negative
            if first_int < 0 || gen < 0 {
                self.diagnostics.push(Diag::with_dynamic_no_offset(
                    DiagCode::StructInvalidIndirectHeader,
                    format!("Invalid indirect reference: {} {} R", first_int, gen),
                ));
                return Some(PdfObject::Null);
            }

            let obj_ref = ObjRef::new(first_int as u32, gen as u16);
            Some(PdfObject::Ref(obj_ref))
        } else {
            // Just a plain integer
            Some(PdfObject::Integer(first_int))
        }
    }

    /// Parse an array: `[ ... ]`
    ///
    /// Arrays can contain any mix of PDF objects.
    /// Returns an empty array on error (with diagnostics).
    fn parse_array(&mut self) -> Option<PdfObject> {
        // Check depth limit
        if self.depth >= MAX_DEPTH {
            self.diagnostics.push(Diag::with_dynamic_no_offset(
                DiagCode::StructDepthExceeded,
                format!("Array nesting depth exceeds limit of {}", MAX_DEPTH),
            ));
            // Skip to matching closing bracket
            self.skip_to_array_end();
            return Some(PdfObject::Null);
        }

        self.depth += 1;
        let mut elements = Vec::new();

        loop {
            match self.lexer.peek_token() {
                Some(Token::ArrayEnd) | Some(Token::Eof) => {
                    // Consume the ArrayEnd token
                    let _ = self.lexer.next_token();
                    break;
                }
                Some(_) => {
                    if let Some(obj) = self.parse_direct_object() {
                        elements.push(obj);
                    } else {
                        // EOF reached
                        break;
                    }
                }
                None => {
                    // Lexer returned None (shouldn't happen after Eof check, but be safe)
                    break;
                }
            }
        }

        self.depth -= 1;
        Some(PdfObject::Array(Box::new(elements)))
    }

    /// Skip tokens until we find an ArrayEnd.
    fn skip_to_array_end(&mut self) {
        loop {
            match self.lexer.next_token() {
                Some(Token::ArrayEnd) | Some(Token::Eof) | None => break,
                Some(_) => continue,
            }
        }
    }

    /// Parse a dictionary: `<< ... >>`
    ///
    /// Dictionaries contain alternating key-value pairs.
    /// Keys must be name objects. Values can be any direct object.
    ///
    /// After parsing the dictionary, check if the next token is `stream`.
    /// If so, parse it as a stream object.
    fn parse_dict(&mut self) -> Option<PdfObject> {
        // Check depth limit
        if self.depth >= MAX_DEPTH {
            self.diagnostics.push(Diag::with_dynamic_no_offset(
                DiagCode::StructDepthExceeded,
                format!("Dictionary nesting depth exceeds limit of {}", MAX_DEPTH),
            ));
            self.skip_to_dict_end();
            return Some(PdfObject::Null);
        }

        self.depth += 1;
        let mut dict = PdfDict::new();
        let mut expecting_key = true;

        loop {
            match self.lexer.peek_token() {
                Some(Token::DictEnd) | Some(Token::Eof) => {
                    // Consume the DictEnd token
                    let _ = self.lexer.next_token();
                    break;
                }
                Some(_) => {
                    if expecting_key {
                        // Parse the key (must be a name)
                        let key_token = self.lexer.next_token()?;
                        match key_token {
                            Token::Name(key_bytes) => {
                                let key_str = String::from_utf8_lossy(&key_bytes);
                                let key = intern(&key_str);

                                // Now parse the value
                                match self.lexer.peek_token() {
                                    Some(Token::DictEnd) | Some(Token::Eof) => {
                                        // Missing value - insert PdfNull
                                        self.diagnostics.push(Diag::with_dynamic_no_offset(
                                            DiagCode::StructInvalidDictValue,
                                            format!("Dictionary key '{}' has no value, inserting null", key),
                                        ));
                                        dict.insert(key, PdfObject::Null);
                                        break; // End of dict
                                    }
                                    Some(_) => {
                                        if let Some(value) = self.parse_direct_object() {
                                            dict.insert(key, value);
                                            expecting_key = true;
                                        } else {
                                            // EOF - end parsing
                                            break;
                                        }
                                    }
                                    None => break,
                                }
                            }
                            _ => {
                                // Invalid key - not a name
                                self.diagnostics.push(Diag::with_dynamic_no_offset(
                                    DiagCode::StructInvalidDictKey,
                                    "Dictionary key is not a name object, skipping".to_string(),
                                ));
                                // Skip the invalid token and the next token (would-be value)
                                let _ = self.lexer.next_token();
                                if !matches!(self.lexer.peek_token(), Some(Token::DictEnd) | Some(Token::Eof) | None) {
                                    let _ = self.lexer.next_token();
                                }
                                expecting_key = true;
                            }
                        }
                    }
                }
                None => break,
            }
        }

        self.depth -= 1;

        // Check if this is followed by `stream` keyword
        if matches!(self.lexer.peek_token(), Some(Token::Stream)) {
            // Consume the stream keyword
            let _ = self.lexer.next_token();

            // Get the stream offset (position after `stream\n`)
            let offset = self.lexer.position();

            // Try to get /Length from the dict
            let len_hint = dict.get("Length").and_then(|obj| obj.as_int()).map(|i| i as u64);

            // Skip the stream body
            self.skip_stream_body(len_hint);

            // Parse the stream object
            return Some(PdfObject::Stream(Box::new(PdfStream::new(dict, offset, len_hint))));
        }

        Some(PdfObject::Dict(Box::new(dict)))
    }

    /// Skip tokens until we find a DictEnd.
    fn skip_to_dict_end(&mut self) {
        loop {
            match self.lexer.next_token() {
                Some(Token::DictEnd) | Some(Token::Eof) | None => break,
                Some(_) => continue,
            }
        }
    }

    /// Skip the stream body.
    ///
    /// If we have a direct length hint, skip that many bytes.
    /// Otherwise, scan for the `endstream` keyword in the raw bytes.
    fn skip_stream_body(&mut self, len_hint: Option<u64>) {
        if let Some(len) = len_hint {
            // Skip the exact number of bytes specified by /Length
            let len_usize = len as usize;
            let actual_skipped = self.lexer.skip_bytes(len);
            if actual_skipped < len_usize {
                self.diagnostics.push(Diag::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedEof,
                    format!("Stream truncated at EOF: expected {} bytes, got {}", len, actual_skipped),
                ));
            }
        } else {
            // No direct length hint - scan for endstream keyword
            self.scan_for_endstream_bytes();
        }

        // After skipping the body, the next token should be EndStream
        match self.lexer.next_token() {
            Some(Token::EndStream) => {
                // Normal case - stream properly terminated
            }
            Some(Token::Eof) => {
                self.diagnostics.push(Diag::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedEof,
                    "Stream truncated at EOF, missing endstream keyword".to_string(),
                ));
            }
            Some(other) => {
                self.diagnostics.push(Diag::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedByte,
                    format!("Expected endstream keyword after stream body, found {:?}", other),
                ));
                // Try to recover by scanning forward for EndStream
                self.scan_to_endstream();
            }
            None => {
                // Shouldn't happen, but handle gracefully
                self.diagnostics.push(Diag::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedEof,
                    "Unexpected None after skipping stream body".to_string(),
                ));
            }
        }
    }

    /// Scan forward in the raw bytes for the `endstream` keyword.
    ///
    /// This is used when /Length is not a direct integer (e.g., an indirect ref).
    fn scan_for_endstream_bytes(&mut self) {
        let remaining = self.lexer.remaining_bytes();
        let pattern = b"endstream";

        // Search for the pattern
        if let Some(pos) = remaining.windows(8).position(|w| w == pattern) {
            // Skip to just before the pattern
            self.lexer.skip_bytes(pos as u64);
        } else {
            // Pattern not found - skip to end
            self.lexer.skip_bytes(remaining.len() as u64);
        }
    }

    /// Scan forward looking for `endstream` keyword.
    fn scan_to_endstream(&mut self) {
        // For now, just keep consuming tokens until we find EndStream or EOF
        loop {
            match self.lexer.next_token() {
                Some(Token::EndStream) | Some(Token::Eof) | None => break,
                Some(_) => continue,
            }
        }
    }

    /// Parse an indirect object: `N G obj ... endobj`
    ///
    /// Indirect objects have the form:
    /// ```text
    /// N G obj
    /// ...direct object...
    /// endobj
    /// ```
    ///
    /// Where N is the object number and G is the generation number.
    ///
    /// # Returns
    /// `Some(PdfIndirect)` on success, `None` on EOF.
    ///
    /// # Error Recovery
    /// - Invalid header (e.g., `1 X obj`): emits `STRUCT_INVALID_INDIRECT_HEADER`,
    ///   scans forward to the next `obj` keyword
    /// - Missing `endobj`: emits `STRUCT_MISSING_KEY`, scans forward to the next
    ///   `endobj`, `obj`, or EOF
    /// - Integer overflow: emits `STRUCT_INTEGER_OVERFLOW`, clamps to max value
    ///
    /// # Example
    ///
    /// ```
    /// use pdftract_core::parser::object::ObjectParser;
    ///
    /// let mut parser = ObjectParser::new(b"1 0 obj\n123\nendobj");
    /// let indirect = parser.parse_indirect_object();
    /// assert!(indirect.is_some());
    /// ```
    pub fn parse_indirect_object(&mut self) -> Option<PdfIndirect> {
        // Read the first token (object number)
        let token1 = self.lexer.next_token()?;

        // Parse the object number
        let obj_num = match token1 {
            Token::Integer(n) => {
                // Check for overflow
                if n > u32::MAX as i64 {
                    self.diagnostics.push(Diag::with_dynamic_no_offset(
                        DiagCode::StructIntegerOverflow,
                        format!("Object number {} exceeds u32::MAX, clamping", n),
                    ));
                    u32::MAX
                } else if n < 0 {
                    self.diagnostics.push(Diag::with_dynamic_no_offset(
                        DiagCode::StructInvalidIndirectHeader,
                        format!("Negative object number {}", n),
                    ));
                    // Recover by scanning forward to next obj keyword
                    self.scan_to_next_obj();
                    return None;
                } else {
                    n as u32
                }
            }
            _ => {
                // Not an integer - emit diagnostic and recover
                self.diagnostics.push(Diag::with_dynamic_no_offset(
                    DiagCode::StructInvalidIndirectHeader,
                    format!("Expected object number, found {:?}", token1),
                ));
                self.scan_to_next_obj();
                return None;
            }
        };

        // Read the second token (generation number)
        let token2 = self.lexer.next_token()?;
        let gen_num = match token2 {
            Token::Integer(g) => {
                // Check for overflow
                if g > u16::MAX as i64 {
                    self.diagnostics.push(Diag::with_dynamic_no_offset(
                        DiagCode::StructIntegerOverflow,
                        format!("Generation number {} exceeds u16::MAX, clamping", g),
                    ));
                    u16::MAX
                } else if g < 0 {
                    self.diagnostics.push(Diag::with_dynamic_no_offset(
                        DiagCode::StructInvalidIndirectHeader,
                        format!("Negative generation number {}", g),
                    ));
                    self.scan_to_next_obj();
                    return None;
                } else {
                    g as u16
                }
            }
            _ => {
                // Not an integer - emit diagnostic and recover
                self.diagnostics.push(Diag::with_dynamic_no_offset(
                    DiagCode::StructInvalidIndirectHeader,
                    format!("Expected generation number, found {:?}", token2),
                ));
                self.scan_to_next_obj();
                return None;
            }
        };

        // Read the third token (must be Obj)
        let token3 = self.lexer.next_token()?;
        if !matches!(token3, Token::Obj) {
            self.diagnostics.push(Diag::with_dynamic_no_offset(
                DiagCode::StructInvalidIndirectHeader,
                format!("Expected 'obj' keyword, found {:?}", token3),
            ));
            self.scan_to_next_obj();
            return None;
        }

        // Construct the ObjRef
        let id = ObjRef::new(obj_num, gen_num);

        // Parse the direct object body
        let obj = self.parse_direct_object().unwrap_or(PdfObject::Null);

        // Expect EndObj token
        match self.lexer.peek_token() {
            Some(Token::EndObj) => {
                // Normal case - consume the EndObj token
                let _ = self.lexer.next_token();
            }
            Some(Token::Obj) => {
                // Found the start of the next indirect object before endobj
                // This means the current object is malformed
                self.diagnostics.push(Diag::with_dynamic_no_offset(
                    DiagCode::StructMissingKey,
                    "Missing 'endobj' before next indirect object".to_string(),
                ));
                // We're positioned at 'obj' but need to be at the object number
                // Scan forward to find the next integer (object number)
                self.scan_to_next_integer();
            }
            Some(Token::Eof) => {
                // Consume the Eof
                let _ = self.lexer.next_token();
                self.diagnostics.push(Diag::with_dynamic_no_offset(
                    DiagCode::StructMissingKey,
                    "Missing 'endobj' at EOF".to_string(),
                ));
            }
            None => {
                self.diagnostics.push(Diag::with_dynamic_no_offset(
                    DiagCode::StructMissingKey,
                    "Missing 'endobj' at EOF".to_string(),
                ));
            }
            Some(_) => {
                // Some other token - scan for endobj or next obj
                self.diagnostics.push(Diag::with_dynamic_no_offset(
                    DiagCode::StructMissingKey,
                    "Expected 'endobj', scanning forward".to_string(),
                ));
                self.scan_to_endobj_or_obj();
            }
        }

        Some(PdfIndirect { id, obj })
    }

    /// Scan forward to the next `obj` keyword for recovery.
    ///
    /// Scans the raw bytes to find the next `obj` keyword without consuming it.
    /// After this call, the lexer is positioned just before the `obj` keyword,
    /// so the next call to `next_token()` will return `Token::Obj`.
    fn scan_to_next_obj(&mut self) {
        let remaining = self.lexer.remaining_bytes();
        let pattern = b"obj";

        // Search for the pattern
        if let Some(pos) = remaining.windows(3).position(|w| w == pattern) {
            // Skip to just before the pattern
            self.lexer.skip_bytes(pos as u64);
        } else {
            // Pattern not found - skip to end
            self.lexer.skip_bytes(remaining.len() as u64);
        }
    }

    /// Scan forward to the next integer for recovery.
    ///
    /// Used when we've detected a missing `endobj` and found the start of the
    /// next indirect object (the `obj` keyword). We need to scan forward to the
    /// next integer (the object number of the next indirect object) so that
    /// the next call to `parse_indirect_object` can correctly parse it.
    ///
    /// After this call, the lexer is positioned just before the next integer token.
    fn scan_to_next_integer(&mut self) {
        let remaining = self.lexer.remaining_bytes();

        // Look for a digit (start of an integer)
        // We need to find a digit preceded by whitespace or at the start
        for (i, &byte) in remaining.iter().enumerate() {
            // Check if this byte could start an integer
            // An integer starts with a digit or a minus sign
            if byte.is_ascii_digit() || byte == b'-' {
                // Check if it's preceded by whitespace or at start
                if i == 0 || remaining[i - 1].is_ascii_whitespace() {
                    // Skip to this position
                    self.lexer.skip_bytes(i as u64);
                    return;
                }
            }
        }

        // No integer found - skip to end
        self.lexer.skip_bytes(remaining.len() as u64);
    }

    /// Scan forward looking for `endobj` or `obj` keyword for recovery.
    ///
    /// Scans the raw bytes to find either keyword and positions the lexer
    /// appropriately:
    /// - If `endobj` is found first: positions lexer after `endobj`
    /// - If `obj` is found first (indicating the next indirect object):
    ///   scans backward to find the preceding integer (the object number)
    ///   and positions the lexer there
    ///
    /// After this call, the lexer is positioned to correctly parse either
    /// the next object or reach EOF.
    fn scan_to_endobj_or_obj(&mut self) {
        let remaining = self.lexer.remaining_bytes();

        // Search for either pattern
        let endobj_pos = remaining.windows(6).position(|w| w == b"endobj");
        let obj_pos = remaining.windows(3).position(|w| w == b"obj");

        // Find the earliest match
        let (min_pos, is_obj) = match (endobj_pos, obj_pos) {
            (Some(e), Some(o)) if e <= o => (Some(e), false),
            (Some(_e), Some(o)) => (Some(o), true),
            (Some(e), None) => (Some(e), false),
            (None, Some(o)) => (Some(o), true),
            (None, None) => (None, false),
        };

        if let Some(pos) = min_pos {
            if is_obj {
                // Found `obj` first - this is the start of the next indirect object
                // We need to scan backward to find the preceding integer (object number)
                // The pattern is: <integer> <integer> obj
                // Scan backward from `obj` to find the start of the first integer
                let mut scan_back = pos;
                // Skip whitespace before `obj`
                while scan_back > 0 && remaining[scan_back - 1].is_ascii_whitespace() {
                    scan_back -= 1;
                }
                // Now we're at the end of the second integer (generation number)
                // Skip the digits of the generation number
                while scan_back > 0 && remaining[scan_back - 1].is_ascii_digit() {
                    scan_back -= 1;
                }
                // Skip whitespace between the two integers
                while scan_back > 0 && remaining[scan_back - 1].is_ascii_whitespace() {
                    scan_back -= 1;
                }
                // Now we're at the end of the first integer (object number)
                // Skip the digits of the object number (and optional minus sign)
                while scan_back > 0 && (remaining[scan_back - 1].is_ascii_digit() || remaining[scan_back - 1] == b'-') {
                    scan_back -= 1;
                }
                // scan_back now points to the start of the object number
                // Skip any remaining whitespace before it
                while scan_back > 0 && remaining[scan_back - 1].is_ascii_whitespace() {
                    scan_back -= 1;
                }
                // Skip to the object number
                self.lexer.skip_bytes(scan_back as u64);
            } else {
                // Found `endobj` first - skip past it
                self.lexer.skip_bytes((pos + 6) as u64);
            }
        } else {
            // Pattern not found - skip to end
            self.lexer.skip_bytes(remaining.len() as u64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_null() {
        let mut parser = ObjectParser::new(b"null");
        let obj = parser.parse_direct_object();
        assert_eq!(obj, Some(PdfObject::Null));
    }

    #[test]
    fn test_parse_bool() {
        let mut parser = ObjectParser::new(b"true");
        let obj = parser.parse_direct_object();
        assert_eq!(obj, Some(PdfObject::Bool(true)));

        let mut parser = ObjectParser::new(b"false");
        let obj = parser.parse_direct_object();
        assert_eq!(obj, Some(PdfObject::Bool(false)));
    }

    #[test]
    fn test_parse_integer() {
        let mut parser = ObjectParser::new(b"123");
        let obj = parser.parse_direct_object();
        assert_eq!(obj, Some(PdfObject::Integer(123)));

        let mut parser = ObjectParser::new(b"-456");
        let obj = parser.parse_direct_object();
        assert_eq!(obj, Some(PdfObject::Integer(-456)));
    }

    #[test]
    fn test_parse_real() {
        let mut parser = ObjectParser::new(b"3.14");
        let obj = parser.parse_direct_object();
        assert_eq!(obj, Some(PdfObject::Real(3.14)));
    }

    #[test]
    fn test_parse_indirect_ref() {
        let mut parser = ObjectParser::new(b"5 0 R");
        let obj = parser.parse_direct_object();
        assert_eq!(obj, Some(PdfObject::Ref(ObjRef::new(5, 0))));

        let mut parser = ObjectParser::new(b"42 3 R");
        let obj = parser.parse_direct_object();
        assert_eq!(obj, Some(PdfObject::Ref(ObjRef::new(42, 3))));
    }

    #[test]
    fn test_parse_string() {
        let mut parser = ObjectParser::new(b"(Hello World)");
        let obj = parser.parse_direct_object();
        // String content is empty in stub lexer, just check type
        assert!(matches!(obj, Some(PdfObject::String(_))));
    }

    #[test]
    fn test_parse_name() {
        let mut parser = ObjectParser::new(b"/Type");
        let obj = parser.parse_direct_object();
        // Name content is empty in stub lexer, just check type
        assert!(matches!(obj, Some(PdfObject::Name(_))));
    }

    #[test]
    fn test_parse_empty_array() {
        let mut parser = ObjectParser::new(b"[ ]");
        let obj = parser.parse_direct_object();
        assert_eq!(obj, Some(PdfObject::Array(Box::new(Vec::new()))));
    }

    #[test]
    fn test_parse_array_of_integers() {
        let mut parser = ObjectParser::new(b"[ 1 2 3 ]");
        let obj = parser.parse_direct_object();
        assert_eq!(obj, Some(PdfObject::Array(Box::new(vec![
            PdfObject::Integer(1),
            PdfObject::Integer(2),
            PdfObject::Integer(3),
        ]))));
    }

    #[test]
    fn test_parse_mixed_array() {
        let mut parser = ObjectParser::new(b"[ 1 true (str) /Name null ]");
        let obj = parser.parse_direct_object();
        if let Some(PdfObject::Array(arr)) = obj {
            assert_eq!(arr.len(), 5);
            assert_eq!(arr[0], PdfObject::Integer(1));
            assert_eq!(arr[1], PdfObject::Bool(true));
            assert!(matches!(arr[2], PdfObject::String(_)));
            assert!(matches!(arr[3], PdfObject::Name(_)));
            assert_eq!(arr[4], PdfObject::Null);
        } else {
            panic!("Expected array, got {:?}", obj);
        }
    }

    #[test]
    fn test_parse_nested_array() {
        let mut parser = ObjectParser::new(b"[ 1 [ 2 3 ] 4 ]");
        let obj = parser.parse_direct_object();
        if let Some(PdfObject::Array(arr)) = obj {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], PdfObject::Integer(1));
            assert_eq!(arr[2], PdfObject::Integer(4));
            if let Some(PdfObject::Array(inner)) = arr.get(1).cloned() {
                assert_eq!(inner.len(), 2);
                assert_eq!(inner[0], PdfObject::Integer(2));
                assert_eq!(inner[1], PdfObject::Integer(3));
            } else {
                panic!("Expected inner array");
            }
        } else {
            panic!("Expected array, got {:?}", obj);
        }
    }

    #[test]
    fn test_parse_empty_dict() {
        let mut parser = ObjectParser::new(b"<< >>");
        let obj = parser.parse_direct_object();
        assert_eq!(obj, Some(PdfObject::Dict(Box::new(PdfDict::new()))));
    }

    #[test]
    fn test_parse_dict() {
        let mut parser = ObjectParser::new(b"<< /Type 1 >>");
        let obj = parser.parse_direct_object();
        if let Some(PdfObject::Dict(dict)) = obj {
            assert_eq!(dict.len(), 1);
            assert!(dict.contains_key("Type"));
        } else {
            panic!("Expected dict, got {:?}", obj);
        }
    }

    #[test]
    fn test_parse_nested_dict() {
        let mut parser = ObjectParser::new(b"<< /A << /B 1 >> >>");
        let obj = parser.parse_direct_object();
        if let Some(PdfObject::Dict(outer)) = obj {
            assert_eq!(outer.len(), 1);
            if let Some(PdfObject::Dict(inner)) = outer.get("A") {
                assert_eq!(inner.len(), 1);
                assert_eq!(inner.get("B"), Some(&PdfObject::Integer(1)));
            } else {
                panic!("Expected inner dict");
            }
        } else {
            panic!("Expected dict, got {:?}", obj);
        }
    }

    #[test]
    fn test_parse_dict_with_missing_value() {
        let mut parser = ObjectParser::new(b"<< /Type >>");
        let obj = parser.parse_direct_object();
        if let Some(PdfObject::Dict(dict)) = obj {
            assert_eq!(dict.len(), 1);
            assert_eq!(dict.get("Type"), Some(&PdfObject::Null));
            let diags = parser.take_diagnostics();
            assert!(diags.iter().any(|d| d.code == DiagCode::StructInvalidDictValue));
        } else {
            panic!("Expected dict, got {:?}", obj);
        }
    }

    #[test]
    fn test_parse_dict_with_invalid_key() {
        let mut parser = ObjectParser::new(b"<< 1 2 >>");
        let obj = parser.parse_direct_object();
        if let Some(PdfObject::Dict(dict)) = obj {
            assert_eq!(dict.len(), 0);
            let diags = parser.take_diagnostics();
            assert!(diags.iter().any(|d| d.code == DiagCode::StructInvalidDictKey));
        } else {
            panic!("Expected dict, got {:?}", obj);
        }
    }

    #[test]
    fn test_position_tracking() {
        let mut parser = ObjectParser::new(b"123");
        assert_eq!(parser.position(), 0);
        parser.parse_direct_object();
        assert!(parser.position() > 0);
    }

    #[test]
    fn test_eof_returns_none() {
        let mut parser = ObjectParser::new(b"123");
        assert!(parser.parse_direct_object().is_some());
        assert!(parser.parse_direct_object().is_none()); // Eof
        assert!(parser.parse_direct_object().is_none()); // Still None
    }

    #[test]
    fn test_parse_4_level_nested_dict() {
        // Critical test from plan: nested dict 4 levels deep -> correct tree
        let input = b"<< /A << /B << /C << /D 1 >> >> >> >>";
        let mut parser = ObjectParser::new(input);
        let obj = parser.parse_direct_object();

        if let Some(PdfObject::Dict(level1)) = obj {
            assert_eq!(level1.len(), 1);
            if let Some(PdfObject::Dict(level2)) = level1.get("A") {
                assert_eq!(level2.len(), 1);
                if let Some(PdfObject::Dict(level3)) = level2.get("B") {
                    assert_eq!(level3.len(), 1);
                    if let Some(PdfObject::Dict(level4)) = level3.get("C") {
                        assert_eq!(level4.len(), 1);
                        assert_eq!(level4.get("D"), Some(&PdfObject::Integer(1)));
                    } else {
                        panic!("Expected level 4 dict");
                    }
                } else {
                    panic!("Expected level 3 dict");
                }
            } else {
                panic!("Expected level 2 dict");
            }
        } else {
            panic!("Expected level 1 dict, got {:?}", obj);
        }
    }

    #[test]
    fn test_depth_exceeded_at_256() {
        // Depth limit: 256 levels - adversarial input protection
        // Create a deeply nested dict (300 levels)
        let mut input = String::from("");
        for _ in 0..300 {
            input.push_str("<< /A ");
        }
        input.push_str("1");
        for _ in 0..300 {
            input.push_str(" >>");
        }

        let mut parser = ObjectParser::new(input.as_bytes());
        let obj = parser.parse_direct_object();

        // At depth 256, the parser returns PdfNull for that level
        // The parent dict (depth 255) receives this and inserts it as a value
        // So we get a dict where at depth 255, key "A" -> PdfNull
        //
        // Navigate 255 levels deep to verify the value is Null
        let mut current = obj.as_ref();
        for _ in 0..255 {
            current = current.and_then(|o| o.as_dict()?.get("A"));
        }
        // After 255 navigations, we should be at the dict at depth 255
        // This dict has key "A" -> PdfNull (because depth 256 hit the limit)
        if let Some(PdfObject::Dict(d)) = current {
            assert_eq!(d.get("A"), Some(&PdfObject::Null));
        } else {
            panic!("Expected dict at depth 255, got {:?}", current);
        }

        // Should have emitted STRUCT_DEPTH_EXCEEDED diagnostic
        let diags = parser.take_diagnostics();
        assert!(diags.iter().any(|d| d.code == DiagCode::StructDepthExceeded));
    }

    #[test]
    fn test_truncated_dict_at_eof() {
        // Truncated dict at EOF -> partial dict + diagnostics
        let input = b"<< /Type /Catalog /Pages";
        let mut parser = ObjectParser::new(input);
        let obj = parser.parse_direct_object();

        // Should get a dict with 2 keys:
        // 1. "Type" -> "/Catalog" (successfully parsed)
        // 2. "Pages" -> PdfNull (missing value, inserted null)
        if let Some(PdfObject::Dict(dict)) = obj {
            assert_eq!(dict.len(), 2);
            assert!(dict.contains_key("Type"));
            assert!(dict.contains_key("Pages"));
            // The Pages key should have PdfNull as value
            assert_eq!(dict.get("Pages"), Some(&PdfObject::Null));
        } else {
            panic!("Expected partial dict, got {:?}", obj);
        }

        // Should have emitted STRUCT_INVALID_DICT_VALUE diagnostic for missing value
        let diags = parser.take_diagnostics();
        assert!(diags.iter().any(|d| d.code == DiagCode::StructInvalidDictValue));
    }

    #[test]
    fn test_negative_indirect_ref() {
        // Invalid indirect reference with negative object number
        let mut parser = ObjectParser::new(b"-1 0 R");
        let obj = parser.parse_direct_object();
        // Should return PdfNull with diagnostic
        assert_eq!(obj, Some(PdfObject::Null));
        let diags = parser.take_diagnostics();
        assert!(diags.iter().any(|d| d.code == DiagCode::StructInvalidIndirectHeader));
    }

    #[test]
    fn test_parse_array_5_elements_mixed_types() {
        // Critical test from plan: array of mixed types -> correct ordering of 5 elements
        let input = b"[1 true (str) /Name null]";
        let mut parser = ObjectParser::new(input);
        let obj = parser.parse_direct_object();

        if let Some(PdfObject::Array(arr)) = obj {
            assert_eq!(arr.len(), 5);
            assert_eq!(arr[0], PdfObject::Integer(1));
            assert_eq!(arr[1], PdfObject::Bool(true));
            assert!(matches!(arr[2], PdfObject::String(_)));
            assert!(matches!(arr[3], PdfObject::Name(_)));
            assert_eq!(arr[4], PdfObject::Null);
        } else {
            panic!("Expected array, got {:?}", obj);
        }
    }

    // proptest property: random valid PDF token sequences never panic (INV-8)
    #[cfg(test)]
    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        /// Strategy to generate random PDF token sequences for fuzzing.
        fn arb_pdf_token_sequence() -> impl Strategy<Value = String> {
            prop_oneof![
                // Simple primitives
                Just("null".to_string()),
                Just("true".to_string()),
                Just("false".to_string()),
                any::<i64>().prop_map(|n| n.to_string()),
                any::<f64>().prop_map(|f| if f.is_finite() { f.to_string() } else { "0.0".to_string() }),
                // Names
                "[a-zA-Z]{1,10}".prop_map(|s| format!("/{}", s)),
                // Strings
                ".*".prop_map(|s| format!("({})", s)),
                // Arrays (simple)
                Just("[1 2 3]".to_string()),
                Just("[]".to_string()),
                // Dicts (simple)
                Just("<< /Type 1 >>".to_string()),
                Just("<< >>".to_string()),
                // Indirect references
                (any::<u32>(), 0..=65535u16).prop_map(|(obj, gen)| format!("{} {} R", obj, gen)),
            ]
        }

        proptest! {
            /// Test that random PDF token sequences never panic (INV-8).
            #[test]
            fn proptest_random_tokens_no_panic(input in arb_pdf_token_sequence()) {
                let bytes = input.as_bytes();
                let mut parser = ObjectParser::new(bytes);
                // Should never panic, may return PdfObject or None
                let _ = parser.parse_direct_object();
                // If we get here without panic, the test passes
            }

            /// Test that random byte sequences never panic (INV-8).
            #[test]
            fn proptest_random_bytes_no_panic(data in any::<Vec<u8>>()) {
                let mut parser = ObjectParser::new(&data);
                // Should never panic, may return PdfObject or None
                let _ = parser.parse_direct_object();
                // If we get here without panic, the test passes
            }
        }
    }

    // Tests for parse_indirect_object

    #[test]
    fn test_parse_indirect_object_simple() {
        // Simple test: `1 0 obj null endobj` -> PdfIndirect{ id: ObjRef{1, 0}, obj: PdfObject::Null }
        let mut parser = ObjectParser::new(b"1 0 obj null endobj");
        let indirect = parser.parse_indirect_object();
        assert!(indirect.is_some());
        let result = indirect.unwrap();
        assert_eq!(result.id, ObjRef::new(1, 0));
        assert_eq!(result.obj, PdfObject::Null);
    }

    #[test]
    fn test_parse_indirect_object_with_integer() {
        let mut parser = ObjectParser::new(b"42 3 obj 123 endobj");
        let indirect = parser.parse_indirect_object();
        assert!(indirect.is_some());
        let result = indirect.unwrap();
        assert_eq!(result.id, ObjRef::new(42, 3));
        assert_eq!(result.obj, PdfObject::Integer(123));
    }

    #[test]
    fn test_parse_indirect_object_with_stream() {
        // Stream test: `12 0 obj << /Length 5 >> stream\n12345endstream endobj`
        let input = b"12 0 obj << /Length 5 >> stream\n12345endstream endobj";
        let mut parser = ObjectParser::new(input);
        let indirect = parser.parse_indirect_object();
        assert!(indirect.is_some());
        let result = indirect.unwrap();
        assert_eq!(result.id, ObjRef::new(12, 0));
        assert!(matches!(result.obj, PdfObject::Stream(_)));
    }

    #[test]
    fn test_parse_indirect_object_missing_endobj() {
        // Recovery test: `1 0 obj null` (no endobj before next `obj`)
        // Should emit STRUCT_MISSING_KEY and position advances
        let input = b"1 0 obj null 2 0 obj 42 endobj";
        let mut parser = ObjectParser::new(input);
        let indirect1 = parser.parse_indirect_object();
        assert!(indirect1.is_some());
        let result1 = indirect1.unwrap();
        assert_eq!(result1.id, ObjRef::new(1, 0));
        assert_eq!(result1.obj, PdfObject::Null);

        // Should have emitted STRUCT_MISSING_KEY diagnostic
        let diags = parser.take_diagnostics();
        assert!(diags.iter().any(|d| d.code == DiagCode::StructMissingKey));

        // Next parse should handle the second object
        let indirect2 = parser.parse_indirect_object();
        assert!(indirect2.is_some());
        let result2 = indirect2.unwrap();
        assert_eq!(result2.id, ObjRef::new(2, 0));
        assert_eq!(result2.obj, PdfObject::Integer(42));
    }

    #[test]
    fn test_parse_indirect_object_integer_overflow() {
        // Recovery test: `999999999999 0 obj null endobj`
        // -> ObjRef{u32::MAX, 0} + STRUCT_INTEGER_OVERFLOW
        let input = b"999999999999 0 obj null endobj";
        let mut parser = ObjectParser::new(input);
        let indirect = parser.parse_indirect_object();
        assert!(indirect.is_some());
        let result = indirect.unwrap();
        assert_eq!(result.id, ObjRef::new(u32::MAX, 0));
        assert_eq!(result.obj, PdfObject::Null);

        // Should have emitted STRUCT_INTEGER_OVERFLOW diagnostic
        let diags = parser.take_diagnostics();
        assert!(diags.iter().any(|d| d.code == DiagCode::StructIntegerOverflow));
    }

    #[test]
    fn test_parse_indirect_object_generation_overflow() {
        let input = b"1 999999999999 obj null endobj";
        let mut parser = ObjectParser::new(input);
        let indirect = parser.parse_indirect_object();
        assert!(indirect.is_some());
        let result = indirect.unwrap();
        assert_eq!(result.id, ObjRef::new(1, u16::MAX));
        assert_eq!(result.obj, PdfObject::Null);

        // Should have emitted STRUCT_INTEGER_OVERFLOW diagnostic
        let diags = parser.take_diagnostics();
        assert!(diags.iter().any(|d| d.code == DiagCode::StructIntegerOverflow));
    }

    #[test]
    fn test_parse_indirect_object_invalid_header() {
        // Invalid header: missing object number
        let input = b"abc 0 obj null endobj";
        let mut parser = ObjectParser::new(input);
        let indirect = parser.parse_indirect_object();
        // Should return None and recover
        assert!(indirect.is_none());

        // Should have emitted STRUCT_INVALID_INDIRECT_HEADER diagnostic
        let diags = parser.take_diagnostics();
        assert!(diags.iter().any(|d| d.code == DiagCode::StructInvalidIndirectHeader));
    }

    #[test]
    fn test_parse_indirect_object_negative_object_number() {
        let input = b"-1 0 obj null endobj";
        let mut parser = ObjectParser::new(input);
        let indirect = parser.parse_indirect_object();
        // Should return None and recover
        assert!(indirect.is_none());

        // Should have emitted STRUCT_INVALID_INDIRECT_HEADER diagnostic
        let diags = parser.take_diagnostics();
        assert!(diags.iter().any(|d| d.code == DiagCode::StructInvalidIndirectHeader));
    }

    #[test]
    fn test_parse_indirect_object_eof_returns_none() {
        let mut parser = ObjectParser::new(b"");
        assert!(parser.parse_indirect_object().is_none());
    }

    #[test]
    fn test_parse_indirect_object_with_dict() {
        let input = b"5 1 obj << /Type /Page >> endobj";
        let mut parser = ObjectParser::new(input);
        let indirect = parser.parse_indirect_object();
        assert!(indirect.is_some());
        let result = indirect.unwrap();
        assert_eq!(result.id, ObjRef::new(5, 1));
        assert!(matches!(result.obj, PdfObject::Dict(_)));
    }

    #[test]
    fn test_parse_indirect_object_with_array() {
        let input = b"10 0 obj [ 1 2 3 ] endobj";
        let mut parser = ObjectParser::new(input);
        let indirect = parser.parse_indirect_object();
        assert!(indirect.is_some());
        let result = indirect.unwrap();
        assert_eq!(result.id, ObjRef::new(10, 0));
        assert!(matches!(result.obj, PdfObject::Array(_)));
    }

    // proptest property: random byte sequences fed to parse_indirect_object never panic
    #[cfg(test)]
    mod proptest_indirect_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Test that random byte sequences never panic when calling parse_indirect_object.
            #[test]
            fn proptest_random_bytes_no_panic_indirect(data in any::<Vec<u8>>()) {
                let mut parser = ObjectParser::new(&data);
                // Should never panic, may return PdfIndirect or None
                let _ = parser.parse_indirect_object();
                // If we get here without panic, the test passes
            }
        }
    }
}
