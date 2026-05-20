//! Diagnostic messages for PDF parsing.
//!
//! This module provides diagnostic types for tracking errors and warnings
//! during PDF parsing, maintaining INV-8 (no panics at public boundaries).

/// Severity level for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Warning - the document can still be processed
    Warning,
    /// Error - recovery attempted, processing continues
    Error,
}

/// Diagnostic code identifying the type of error or warning.
///
/// These codes provide structured error classification for diagnostics
/// emitted during PDF parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagCode {
    // Lexer codes
    /// Invalid name character or malformed name
    StructInvalidName,
    /// Invalid hexadecimal character in hex string or name escape
    StructInvalidHex,
    /// Invalid octal escape sequence in literal string
    StructInvalidOctal,
    /// Invalid stream header (stream keyword not followed by proper newline)
    StructInvalidStreamHeader,
    /// Unexpected end of file while parsing a token
    StructUnexpectedEof,
    /// Unterminated literal string (missing closing paren)
    StructUnterminatedString,

    // Object parser codes
    /// Dictionary nesting depth exceeds limit
    DepthExceeded,
    /// Invalid dictionary value (missing value after key)
    InvalidDictValue,
    /// Invalid dictionary key (not a name object)
    InvalidDictKey,
    /// Invalid indirect object header
    InvalidIndirectHeader,
    /// Integer overflow during parsing
    IntegerOverflow,
    /// Missing required key in dictionary
    MissingKey,

    // Object stream codes
    /// Invalid object stream format
    InvalidObjstm,
    /// Circular reference in /Extends chain
    CircularRef,
    /// Stream decompression failed
    DecompressionFailed,
    /// Decompression bomb limit exceeded
    StreamBomb,
    /// Unsupported encryption (custom crypt filter, unknown encryption handler)
    EncryptionUnsupported,

    // Page tree codes
    /// Invalid page count
    InvalidPageCount,
    /// Invalid rotate value (not multiple of 90)
    InvalidRotate,

    // Outline codes
    /// Invalid UTF-16BE encoding in string
    StructInvalidUtf16,
    /// Named destination cannot be resolved (requires /Names /Dests lookup)
    StructUnresolvedDestination,
    /// Outline action is not a GoTo action (e.g., URI action)
    StructNonGotoOutline,
}

/// A diagnostic message emitted during PDF parsing.
///
/// Per INV-8, all errors are emitted as diagnostics rather than panicking.
/// The parser always attempts recovery and continues processing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Diagnostic code identifying the type of error
    pub code: DiagCode,
    /// Severity level
    pub severity: Severity,
    /// Phase identifier (e.g., "1.4" for document model)
    pub phase: String,
    /// Human-readable message
    pub message: String,
}

impl Diagnostic {
    /// Create a new diagnostic.
    pub fn new(severity: Severity, phase: impl Into<String>, message: impl Into<String>) -> Self {
        Diagnostic {
            code: DiagCode::StructUnexpectedEof, // Default code
            severity,
            phase: phase.into(),
            message: message.into(),
        }
    }

    /// Create a new diagnostic with a specific code.
    pub fn new_with_code(code: DiagCode, severity: Severity, phase: impl Into<String>, message: impl Into<String>) -> Self {
        Diagnostic {
            code,
            severity,
            phase: phase.into(),
            message: message.into(),
        }
    }

    /// Create a warning diagnostic.
    pub fn warning(phase: impl Into<String>, message: impl Into<String>) -> Self {
        Diagnostic {
            code: DiagCode::StructUnexpectedEof, // Default code
            severity: Severity::Warning,
            phase: phase.into(),
            message: message.into(),
        }
    }

    /// Create an error diagnostic.
    pub fn error(phase: impl Into<String>, message: impl Into<String>) -> Self {
        Diagnostic {
            code: DiagCode::StructUnexpectedEof, // Default code
            severity: Severity::Error,
            phase: phase.into(),
            message: message.into(),
        }
    }

    /// Create an error diagnostic with a specific code.
    pub fn error_with_code(code: DiagCode, phase: impl Into<String>, message: impl Into<String>) -> Self {
        Diagnostic {
            code,
            severity: Severity::Error,
            phase: phase.into(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_new() {
        let diag = Diagnostic::new(Severity::Error, "1.4", "test message");
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.phase, "1.4");
        assert_eq!(diag.message, "test message");
    }

    #[test]
    fn test_diagnostic_warning() {
        let diag = Diagnostic::warning("1.4", "test warning");
        assert_eq!(diag.severity, Severity::Warning);
    }

    #[test]
    fn test_diagnostic_error() {
        let diag = Diagnostic::error("1.4", "test error");
        assert_eq!(diag.severity, Severity::Error);
    }
}
