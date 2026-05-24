//! Diagnostic assertion helpers for xref tests.
//!
//! Provides helpers for asserting that specific diagnostics were emitted
//! during xref parsing, with support for byte offset range matching.

use pdftract_core::diagnostics::{DiagCode, Diagnostic};
use std::ops::RangeInclusive;

/// Assert that a specific diagnostic code was emitted.
///
/// # Parameters
/// - `diagnostics`: The diagnostics emitted during parsing
/// - `code`: The expected diagnostic code
///
/// # Panics
/// Panics if the diagnostic code is not found in the diagnostics list.
pub fn assert_diagnostic(diagnostics: &[Diagnostic], code: DiagCode) {
    let found = diagnostics.iter().any(|d| d.code == code);
    if !found {
        panic!(
            "Expected diagnostic {:?} not found. Got: {:?}",
            code,
            diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
        );
    }
}

/// Assert that a specific diagnostic code was emitted with a byte offset in range.
///
/// # Parameters
/// - `diagnostics`: The diagnostics emitted during parsing
/// - `code`: The expected diagnostic code
/// - `byte_offset_range`: Inclusive range of acceptable byte offsets
///
/// # Panics
/// Panics if:
/// - The diagnostic code is not found
/// - The diagnostic is found but has no byte offset
/// - The byte offset is outside the expected range
pub fn assert_diagnostic_in_range(
    diagnostics: &[Diagnostic],
    code: DiagCode,
    byte_offset_range: RangeInclusive<u64>,
) {
    let matching = diagnostics
        .iter()
        .filter(|d| d.code == code)
        .collect::<Vec<_>>();

    if matching.is_empty() {
        panic!(
            "Expected diagnostic {:?} not found. Got: {:?}",
            code,
            diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
        );
    }

    let found = matching.iter().find(|d| {
        if let Some(offset) = d.byte_offset {
            byte_offset_range.contains(&offset)
        } else {
            false
        }
    });

    if found.is_none() {
        let offsets = matching
            .iter()
            .filter_map(|d| d.byte_offset)
            .collect::<Vec<_>>();
        panic!(
            "Diagnostic {:?} found but byte offset {:?} not in range {:?}",
            code, offsets, byte_offset_range
        );
    }
}

/// Assert that a specific diagnostic code was emitted a specific number of times.
///
/// # Parameters
/// - `diagnostics`: The diagnostics emitted during parsing
/// - `code`: The expected diagnostic code
/// - `count`: The expected number of occurrences
///
/// # Panics
/// Panics if the diagnostic code does not appear exactly `count` times.
pub fn assert_diagnostic_count(diagnostics: &[Diagnostic], code: DiagCode, count: usize) {
    let actual = diagnostics.iter().filter(|d| d.code == code).count();
    if actual != count {
        panic!(
            "Expected diagnostic {:?} to appear {} times, but found {} times",
            code, count, actual
        );
    }
}

/// Assert that NO diagnostics with the given severity level were emitted.
///
/// # Parameters
/// - `diagnostics`: The diagnostics emitted during parsing
/// - `severity`: The severity level that should not appear
///
/// # Panics
/// Panics if any diagnostic with the given severity is found.
pub fn assert_no_diagnostic_with_severity(
    diagnostics: &[Diagnostic],
    severity: pdftract_core::diagnostics::Severity,
) {
    let found: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity() == severity)
        .collect();

    if !found.is_empty() {
        panic!(
            "Expected no {:?} diagnostics, but found {:?}",
            severity,
            found.iter().map(|d| d.code).collect::<Vec<_>>()
        );
    }
}

/// Count diagnostics by code.
///
/// # Parameters
/// - `diagnostics`: The diagnostics emitted during parsing
/// - `code`: The diagnostic code to count
///
/// # Returns
/// The number of diagnostics with the given code.
pub fn count_diagnostics(diagnostics: &[Diagnostic], code: DiagCode) -> usize {
    diagnostics.iter().filter(|d| d.code == code).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdftract_core::diagnostics::DiagCode;

    #[test]
    fn test_assert_diagnostic_passes() {
        let diagnostics = vec![Diagnostic::with_static(DiagCode::StructInvalidName, 100, "test")];
        // Should not panic
        assert_diagnostic(&diagnostics, DiagCode::StructInvalidName);
    }

    #[test]
    #[should_panic]
    fn test_assert_diagnostic_panics() {
        let diagnostics = vec![Diagnostic::with_static(DiagCode::StructInvalidName, 100, "test")];
        assert_diagnostic(&diagnostics, DiagCode::StructInvalidHex);
    }

    #[test]
    fn test_assert_diagnostic_in_range_passes() {
        let diagnostics = vec![Diagnostic::with_static(DiagCode::StructInvalidName, 100, "test")];
        // Should not panic
        assert_diagnostic_in_range(&diagnostics, DiagCode::StructInvalidName, 50..=150);
    }

    #[test]
    #[should_panic]
    fn test_assert_diagnostic_in_range_panics() {
        let diagnostics = vec![Diagnostic::with_static(DiagCode::StructInvalidName, 100, "test")];
        assert_diagnostic_in_range(&diagnostics, DiagCode::StructInvalidName, 150..=200);
    }

    #[test]
    fn test_assert_diagnostic_count_passes() {
        let diagnostics = vec![
            Diagnostic::with_static(DiagCode::StructInvalidName, 100, "test1"),
            Diagnostic::with_static(DiagCode::StructInvalidName, 200, "test2"),
        ];
        // Should not panic
        assert_diagnostic_count(&diagnostics, DiagCode::StructInvalidName, 2);
    }

    #[test]
    #[should_panic]
    fn test_assert_diagnostic_count_panics() {
        let diagnostics = vec![
            Diagnostic::with_static(DiagCode::StructInvalidName, 100, "test1"),
            Diagnostic::with_static(DiagCode::StructInvalidName, 200, "test2"),
        ];
        assert_diagnostic_count(&diagnostics, DiagCode::StructInvalidName, 1);
    }
}
