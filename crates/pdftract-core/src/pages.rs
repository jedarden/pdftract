//! Page range parsing for selective extraction.
//!
//! This module provides functionality for parsing page range specifications
//! in the format accepted by the --pages CLI flag. Ranges are 1-based and
//! comma-separated, with support for open-ended ranges like "1-5,7,12-".
//!
//! # Syntax
//!
//! - Single pages: `1`, `3`, `7`
//! - Closed ranges: `1-5` (pages 1, 2, 3, 4, 5)
//! - Open-start ranges: `-5` (pages 1, 2, 3, 4, 5 - equivalent to 1-5)
//! - Open-end ranges: `12-` (pages 12 through end of document)
//! - Comma-separated: `1-5,7,12-15`
//!
//! # Examples
//!
//! ```
//! use pdftract_core::pages::parse_pages;
//! use std::collections::BTreeSet;
//!
//! // Closed range
//! let indices = parse_pages("1-5", 10).unwrap();
//! assert_eq!(indices, BTreeSet::from([0, 1, 2, 3, 4]));
//!
//! // Single pages
//! let indices = parse_pages("1,3,7", 10).unwrap();
//! assert_eq!(indices, BTreeSet::from([0, 2, 6]));
//!
//! // Open-end range
//! let indices = parse_pages("8-", 10).unwrap();
//! assert_eq!(indices, BTreeSet::from([7, 8, 9]));
//!
//! // Open-start range (equivalent to 1-5)
//! let indices = parse_pages("-5", 10).unwrap();
//! assert_eq!(indices, BTreeSet::from([0, 1, 2, 3, 4]));
//! ```

use crate::diagnostics::{Diagnostic, DiagCode};
use std::collections::BTreeSet;

/// Error type for page range parsing failures.
///
/// These errors represent malformed range syntax (non-integer values,
/// invalid characters, etc.) and should result in CLI exit code 2.
/// Out-of-range pages are NOT errors - they emit diagnostics instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageRangeError {
    /// Empty range string (no pages specified)
    EmptyRange,
    /// Invalid integer in range specification
    InvalidInteger(String),
    /// Page number is zero (1-based indexing)
    ZeroPageNumber,
    /// Malformed range syntax (e.g., "1--5", "1-", "-")
    MalformedRange(String),
}

impl std::fmt::Display for PageRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PageRangeError::EmptyRange => {
                write!(f, "page range cannot be empty")
            }
            PageRangeError::InvalidInteger(s) => {
                write!(f, "invalid integer in page range: '{}'", s)
            }
            PageRangeError::ZeroPageNumber => {
                write!(f, "page numbers are 1-based; zero is not allowed")
            }
            PageRangeError::MalformedRange(s) => {
                write!(f, "malformed page range: '{}'", s)
            }
        }
    }
}

impl std::error::Error for PageRangeError {}

/// Parse a page range specification into a set of 0-based page indices.
///
/// The range string uses 1-based page numbers (user-facing) but returns
/// 0-based indices (internal). Out-of-range page numbers emit diagnostics
/// but do not cause parsing to fail - they are simply skipped.
///
/// # Arguments
///
/// * `range` - The range specification string (e.g., "1-5,7,12-")
/// * `page_count` - The total number of pages in the document
/// * `diagnostics` - Output vector for PAGE_OUT_OF_RANGE diagnostics
///
/// # Returns
///
/// A `BTreeSet` of 0-based page indices, sorted and deduplicated.
///
/// # Examples
///
/// ```
/// use pdftract_core::pages::parse_pages;
/// use std::collections::BTreeSet;
///
/// let mut diagnostics = Vec::new();
/// let indices = parse_pages("1-3,7", 10, &mut diagnostics).unwrap();
/// assert_eq!(indices, BTreeSet::from([0, 1, 2, 6]));
/// assert!(diagnostics.is_empty());
///
/// // Out-of-range page emits diagnostic but doesn't fail
/// let indices = parse_pages("12", 10, &mut diagnostics).unwrap();
/// assert!(indices.is_empty());
/// assert_eq!(diagnostics.len(), 1);
/// assert_eq!(diagnostics[0].code, DiagCode::PageOutOfRange);
/// ```
pub fn parse_pages(
    range: &str,
    page_count: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<BTreeSet<usize>, PageRangeError> {
    if range.trim().is_empty() {
        return Err(PageRangeError::EmptyRange);
    }

    let mut indices = BTreeSet::new();

    for part in range.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err(PageRangeError::MalformedRange(
                "empty part in comma-separated range".to_string(),
            ));
        }

        // Check for malformed ranges with multiple dashes (e.g., "1--5")
        // by counting dashes - if more than one, it's malformed
        let dash_count = part.chars().filter(|&c| c == '-').count();
        if dash_count > 1 {
            return Err(PageRangeError::MalformedRange(
                format!("range '{}' has multiple dashes", part),
            ));
        }

        // Check if this is a range (contains '-')
        if dash_count == 1 {
            // Find the dash position
            let dash_idx = part.find('-').unwrap();
            let start = &part[..dash_idx];
            let end = &part[dash_idx + 1..];

            // Both parts empty: "-" is malformed
            if start.is_empty() && end.is_empty() {
                return Err(PageRangeError::MalformedRange(
                    "range '-' has no start or end".to_string(),
                ));
            }

            // Parse start (default to 1 if empty for open-start ranges)
            let s = if start.is_empty() {
                1
            } else {
                start
                    .trim()
                    .parse()
                    .map_err(|_| PageRangeError::InvalidInteger(start.trim().to_string()))?
            };

            // Parse end (default to page_count if empty for open-end ranges)
            let e = if end.is_empty() {
                page_count
            } else {
                end.trim()
                    .parse()
                    .map_err(|_| PageRangeError::InvalidInteger(end.trim().to_string()))?
            };

            // Validate: zero not allowed in 1-based indexing
            if s == 0 || e == 0 {
                return Err(PageRangeError::ZeroPageNumber);
            }

            // Check if start > end
            if s > e {
                // Emit a single diagnostic for the out-of-range start
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::PageOutOfRange,
                    format!("page {} exceeds document page count ({})", s, page_count),
                ));
                continue;
            }

            // Add each page in the range (inclusive)
            // Note: We don't emit an early diagnostic for s > page_count here
            // because the loop below will emit diagnostics for all out-of-range
            // pages including the start. This avoids duplicate diagnostics.
            for n in s..=e {
                if n > page_count {
                    // Emit diagnostic for out-of-range page
                    diagnostics.push(Diagnostic::with_dynamic_no_offset(
                        DiagCode::PageOutOfRange,
                        format!("page {} exceeds document page count ({})", n, page_count),
                    ));
                    continue;
                }
                if n == 0 {
                    // Zero is invalid in 1-based indexing
                    continue;
                }
                // Convert 1-based to 0-based
                indices.insert(n - 1);
            }
        } else {
            // Single page number
            let n: usize = part
                .parse()
                .map_err(|_| PageRangeError::InvalidInteger(part.to_string()))?;

            if n == 0 {
                return Err(PageRangeError::ZeroPageNumber);
            }

            if n > page_count {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::PageOutOfRange,
                    format!("page {} exceeds document page count ({})", n, page_count),
                ));
                continue;
            }
            // Convert 1-based to 0-based
            indices.insert(n - 1);
        }
    }

    Ok(indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Severity;

    #[test]
    fn test_parse_single_page() {
        let mut diagnostics = Vec::new();
        let indices = parse_pages("1", 10, &mut diagnostics).unwrap();
        assert_eq!(indices, BTreeSet::from([0]));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_parse_multiple_single_pages() {
        let mut diagnostics = Vec::new();
        let indices = parse_pages("1,3,7", 10, &mut diagnostics).unwrap();
        assert_eq!(indices, BTreeSet::from([0, 2, 6]));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_parse_closed_range() {
        let mut diagnostics = Vec::new();
        let indices = parse_pages("1-5", 10, &mut diagnostics).unwrap();
        assert_eq!(indices, BTreeSet::from([0, 1, 2, 3, 4]));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_parse_open_end_range() {
        let mut diagnostics = Vec::new();
        let indices = parse_pages("8-", 10, &mut diagnostics).unwrap();
        assert_eq!(indices, BTreeSet::from([7, 8, 9]));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_parse_open_start_range() {
        let mut diagnostics = Vec::new();
        let indices = parse_pages("-5", 10, &mut diagnostics).unwrap();
        assert_eq!(indices, BTreeSet::from([0, 1, 2, 3, 4]));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_parse_mixed_ranges() {
        let mut diagnostics = Vec::new();
        let indices = parse_pages("1-3,7,9-", 10, &mut diagnostics).unwrap();
        // 1-3 → pages 1,2,3 → 0-based: 0,1,2
        // 7 → page 7 → 0-based: 6
        // 9- → pages 9,10 → 0-based: 8,9
        assert_eq!(indices, BTreeSet::from([0, 1, 2, 6, 8, 9]));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_parse_out_of_range_single() {
        let mut diagnostics = Vec::new();
        let indices = parse_pages("12", 10, &mut diagnostics).unwrap();
        assert!(indices.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::PageOutOfRange);
        assert_eq!(diagnostics[0].severity(), Severity::Error);
    }

    #[test]
    fn test_parse_out_of_range_in_closed_range() {
        let mut diagnostics = Vec::new();
        let indices = parse_pages("8-12", 10, &mut diagnostics).unwrap();
        // Should get pages 7, 8, 9 (0-based: 7, 8, 9) but not 10, 11
        assert_eq!(indices, BTreeSet::from([7, 8, 9]));
        assert_eq!(diagnostics.len(), 2); // pages 11 and 12 are out of range
    }

    #[test]
    fn test_parse_out_of_range_open_end() {
        let mut diagnostics = Vec::new();
        let indices = parse_pages("12-", 10, &mut diagnostics).unwrap();
        assert!(indices.is_empty());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_parse_empty_range() {
        let mut diagnostics = Vec::new();
        let result = parse_pages("", 10, &mut diagnostics);
        assert_eq!(result, Err(PageRangeError::EmptyRange));
    }

    #[test]
    fn test_parse_zero_page() {
        let mut diagnostics = Vec::new();
        let result = parse_pages("0", 10, &mut diagnostics);
        assert_eq!(result, Err(PageRangeError::ZeroPageNumber));
    }

    #[test]
    fn test_parse_invalid_integer() {
        let mut diagnostics = Vec::new();
        let result = parse_pages("abc", 10, &mut diagnostics);
        assert!(matches!(result, Err(PageRangeError::InvalidInteger(_))));
    }

    #[test]
    fn test_parse_malformed_range_double_dash() {
        let mut diagnostics = Vec::new();
        let result = parse_pages("1--5", 10, &mut diagnostics);
        assert!(matches!(result, Err(PageRangeError::MalformedRange(_))));
    }

    #[test]
    fn test_parse_malformed_range_only_dash() {
        let mut diagnostics = Vec::new();
        let result = parse_pages("-", 10, &mut diagnostics);
        assert!(matches!(result, Err(PageRangeError::MalformedRange(_))));
    }

    #[test]
    fn test_parse_whitespace_tolerance() {
        let mut diagnostics = Vec::new();
        let indices = parse_pages(" 1 - 3 , 7 ", 10, &mut diagnostics).unwrap();
        assert_eq!(indices, BTreeSet::from([0, 1, 2, 6]));
    }

    #[test]
    fn test_parse_deduplication() {
        let mut diagnostics = Vec::new();
        // 1-5 includes 3, so 3 should only appear once
        let indices = parse_pages("1-5,3,7", 10, &mut diagnostics).unwrap();
        assert_eq!(indices, BTreeSet::from([0, 1, 2, 3, 4, 6]));
    }

    #[test]
    fn test_parse_sorted_output() {
        let mut diagnostics = Vec::new();
        // Input in non-sorted order
        let indices = parse_pages("7,1,5,3", 10, &mut diagnostics).unwrap();
        // BTreeSet guarantees sorted order
        let sorted: Vec<_> = indices.iter().copied().collect();
        assert_eq!(sorted, vec![0, 2, 4, 6]);
    }

    #[test]
    fn test_parse_complex_example() {
        let mut diagnostics = Vec::new();
        // From the acceptance criteria: 1-5,7,12-15 with page_count=10
        let indices = parse_pages("1-5,7,12-15", 10, &mut diagnostics).unwrap();
        assert_eq!(indices, BTreeSet::from([0, 1, 2, 3, 4, 6]));
        assert_eq!(diagnostics.len(), 4); // pages 12, 13, 14, 15
    }
}
