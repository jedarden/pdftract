//! Diagnostic messages for PDF parsing.
//!
//! This module provides diagnostic types for tracking errors and warnings
//! during PDF parsing, maintaining INV-8 (no panics at public boundaries).

/// Severity level for diagnostics, shared with the code policy and JSON contract.
pub use crate::diagnostics::Severity;

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

    // Graphics state codes
    /// T* operator when leading == 0 (no-op, no vertical movement)
    TstarZeroLeading,
    /// Tf operator referenced font name not found in resource dictionary
    FontResourceNotFound,
    /// Tf operator with font_size <= 0 (clamped to 1.0)
    FontSizeZeroOrNegative,
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
    pub fn new_with_code(
        code: DiagCode,
        severity: Severity,
        phase: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
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
    pub fn error_with_code(
        code: DiagCode,
        phase: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
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
    use crate::diagnostics::DiagCode as CanonicalDiagCode;

    #[test]
    fn severity_uses_canonical_policy_and_wire_spellings() {
        for (code, expected) in [
            (CanonicalDiagCode::XrefRepaired, "info"),
            (CanonicalDiagCode::StructInvalidName, "warning"),
            (CanonicalDiagCode::StreamBomb, "error"),
            (CanonicalDiagCode::EncryptionUnsupported, "fatal"),
        ] {
            // The policy value must be usable directly in a typed parser diagnostic.
            let severity: Severity = code.policy().severity;
            let diagnostic = Diagnostic::new(severity, "test", "test message");
            assert_eq!(diagnostic.severity.to_string(), expected);

            let canonical =
                crate::diagnostics::Diagnostic::with_static_no_offset(code, "test message");
            let wire = crate::schema::DiagnosticJson::from(&canonical);
            assert_eq!(canonical.severity(), diagnostic.severity);
            assert_eq!(wire.severity, expected);
        }
    }

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
