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

/// A diagnostic message emitted during PDF parsing.
///
/// Per INV-8, all errors are emitted as diagnostics rather than panicking.
/// The parser always attempts recovery and continues processing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
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
            severity,
            phase: phase.into(),
            message: message.into(),
        }
    }

    /// Create a warning diagnostic.
    pub fn warning(phase: impl Into<String>, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            phase: phase.into(),
            message: message.into(),
        }
    }

    /// Create an error diagnostic.
    pub fn error(phase: impl Into<String>, message: impl Into<String>) -> Self {
        Diagnostic {
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
