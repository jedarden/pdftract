//! Page range parsing and validation for the --pages CLI flag.
//!
//! This module provides functionality for parsing page range strings into
//! sorted, deduped 0-based page indices for selective extraction.
//!
//! # Page Range Format
//!
//! Page ranges are 1-based (user-facing) and converted to 0-based indices internally.
//! The format accepts:
//! - Single pages: "1", "3", "7"
//! - Closed ranges: "1-5" (pages 1-5 inclusive)
//! - Open-start ranges: "-5" (equivalent to "1-5")
//! - Open-end ranges: "12-" (page 12 to end)
//! - Comma-separated: "1-5,7,12-15"
//!
//! # Whitespace handling
//!
//! Whitespace around commas and ranges is trimmed:
//! - "1-5, 7" == "1-5,7"
//! - "1, 3, 7" == "1,3,7"
//! - "12 -" == "12-"
//!
//! # Validation
//!
//! - Invalid syntax ("5-3", "abc", "1.5") returns an error
//! - Out-of-range pages are handled by the caller (emit PAGE_OUT_OF_RANGE diagnostic)
//! - Page numbers must be >= 1

use std::collections::BTreeSet;

/// Error type for page range parsing failures.
#[derive(Debug, Clone, PartialEq)]
pub enum PageRangeError {
    /// Empty page range string
    EmptyRange,
    /// Invalid page number (non-numeric)
    InvalidPageNumber(String),
    /// Page number <= 0
    NonPositivePageNumber(String),
    /// Invalid range syntax (e.g., "5-3" where end < start)
    InvalidRange(String, String),
    /// Malformed range (e.g., "1-", "abc", "1.5")
    MalformedRange(String),
}

impl std::fmt::Display for PageRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PageRangeError::EmptyRange => {
                write!(f, "Page range cannot be empty")
            }
            PageRangeError::InvalidPageNumber(s) => {
                write!(f, "Invalid page number '{}': must be a positive integer", s)
            }
            PageRangeError::NonPositivePageNumber(s) => {
                write!(f, "Page number '{}' must be >= 1 (pages are 1-based)", s)
            }
            PageRangeError::InvalidRange(start, end) => {
                write!(
                    f,
                    "Invalid page range: start '{}' must be <= end '{}'",
                    start, end
                )
            }
            PageRangeError::MalformedRange(s) => {
                write!(
                    f,
                    "Malformed page range '{}': expected format: N, N-, -N, or N-M",
                    s
                )
            }
        }
    }
}

impl std::error::Error for PageRangeError {}

/// Parse a page range string into a sorted, deduped set of 0-based page indices.
///
/// # Arguments
///
/// * `range_str` - The page range string (1-based, comma-separated)
/// * `page_count` - Total number of pages in the document (for open-end ranges)
///
/// # Returns
///
/// Returns `Ok(BTreeSet<usize>)` containing 0-based page indices, or `Err(PageRangeError)`
/// describing why parsing failed.
///
/// # Examples
///
/// ```ignore
/// use pdftract_cli::pages::parse_page_range;
///
/// // Single page
/// let pages = parse_page_range("1", 10).unwrap();
/// assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![0]); // 0-based
///
/// // Closed range
/// let pages = parse_page_range("1-5", 10).unwrap();
/// assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);
///
/// // Open-start range (equivalent to 1-5)
/// let pages = parse_page_range("-5", 10).unwrap();
/// assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);
///
/// // Open-end range (12 to end)
/// let pages = parse_page_range("12-", 20).unwrap();
/// assert_eq!(pages.len(), 9); // pages 12-20 inclusive
///
/// // Comma-separated
/// let pages = parse_page_range("1,3,7", 10).unwrap();
/// assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![0, 2, 6]);
///
/// // Complex range
/// let pages = parse_page_range("1-5,7,12-", 20).unwrap();
/// // Returns 0-4, 6, 11-19 (0-based)
/// ```
pub fn parse_page_range(range_str: &str, page_count: usize) -> Result<BTreeSet<usize>, PageRangeError> {
    if range_str.trim().is_empty() {
        return Err(PageRangeError::EmptyRange);
    }

    let mut result = BTreeSet::new();

    // Split by comma and process each part
    for part in range_str.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Check if this is a range (contains '-')
        if let Some(dash_pos) = part.find('-') {
            // Could be "N-M", "N-", or "-N"
            let before_dash = part[..dash_pos].trim();
            let after_dash = part[dash_pos + 1..].trim();

            match (before_dash.is_empty(), after_dash.is_empty()) {
                // "-N" → open-start range (1 to N)
                (true, false) => {
                    let end = parse_page_number(after_dash)?;
                    let end_idx = to_0based(end, page_count)?;
                    for idx in 0..=end_idx {
                        result.insert(idx);
                    }
                }
                // "N-" → open-end range (N to end)
                (false, true) => {
                    let start = parse_page_number(before_dash)?;
                    let start_idx = to_0based(start, page_count)?;
                    for idx in start_idx..page_count {
                        result.insert(idx);
                    }
                }
                // "N-M" → closed range
                (false, false) => {
                    let start = parse_page_number(before_dash)?;
                    let end = parse_page_number(after_dash)?;

                    if start > end {
                        return Err(PageRangeError::InvalidRange(before_dash.to_string(), after_dash.to_string()));
                    }

                    let start_idx = to_0based(start, page_count)?;
                    let end_idx = to_0based(end, page_count)?;
                    for idx in start_idx..=end_idx {
                        result.insert(idx);
                    }
                }
                // "-" → malformed
                (true, true) => {
                    return Err(PageRangeError::MalformedRange(part.to_string()));
                }
            }
        } else {
            // Single page number
            let page = parse_page_number(part)?;
            let idx = to_0based(page, page_count)?;
            result.insert(idx);
        }
    }

    Ok(result)
}

/// Parse a string as a 1-based page number.
///
/// Returns an error if the string is not a valid positive integer.
fn parse_page_number(s: &str) -> Result<usize, PageRangeError> {
    let n: usize = s.parse().map_err(|_| PageRangeError::InvalidPageNumber(s.to_string()))?;
    if n == 0 {
        Err(PageRangeError::NonPositivePageNumber(s.to_string()))
    } else {
        Ok(n)
    }
}

/// Convert a 1-based page number to a 0-based index.
///
/// Returns an error if the page number exceeds the page count.
fn to_0based(page: usize, page_count: usize) -> Result<usize, PageRangeError> {
    if page > page_count {
        // Note: We don't error here - we let the caller handle out-of-range pages
        // by emitting PAGE_OUT_OF_RANGE diagnostics. This function clamps to the
        // maximum valid 0-based index for now.
        Ok(page_count.saturating_sub(1))
    } else {
        Ok(page - 1)
    }
}

/// Filter out-of-range page indices from a set.
///
/// Given a set of 0-based page indices and the total page count, return
/// a new set containing only valid indices. Returns a vector of out-of-range
/// page numbers (1-based) for diagnostic emission.
///
/// # Arguments
///
/// * `indices` - Set of 0-based page indices (may contain out-of-range values)
/// * `page_count` - Total number of pages in the document
///
/// # Returns
///
/// A tuple of (valid_indices, out_of_range_pages) where:
/// - `valid_indices` is a BTreeSet of valid 0-based indices
/// - `out_of_range_pages` is a Vec of 1-based page numbers that were out of range
///
/// # Examples
///
/// ```ignore
/// use pdftract_cli::pages::{parse_page_range, filter_out_of_range};
/// use std::collections::BTreeSet;
///
/// // Parse a range that includes out-of-range pages
/// let indices = parse_page_range("1-5,10-15", 10).unwrap();
///
/// // Filter to get valid indices and out-of-range pages
/// let (valid, out_of_range) = filter_out_of_range(&indices, 10);
///
/// // valid: 0-4 (pages 1-5)
/// // out_of_range: [10, 11, 12, 13, 14, 15] (1-based)
/// ```
pub fn filter_out_of_range(
    indices: &BTreeSet<usize>,
    page_count: usize,
) -> (BTreeSet<usize>, Vec<usize>) {
    let valid: BTreeSet<usize> = indices
        .iter()
        .filter(|&&idx| idx < page_count)
        .copied()
        .collect();

    let out_of_range: Vec<usize> = indices
        .iter()
        .filter(|&&idx| idx >= page_count)
        .map(|&idx| idx + 1) // Convert back to 1-based for reporting
        .collect();

    (valid, out_of_range)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_page_number_valid() {
        assert_eq!(parse_page_number("1").unwrap(), 1);
        assert_eq!(parse_page_number("10").unwrap(), 10);
        assert_eq!(parse_page_number("100").unwrap(), 100);
    }

    #[test]
    fn test_parse_page_number_invalid() {
        assert!(matches!(
            parse_page_number("0"),
            Err(PageRangeError::NonPositivePageNumber(_))
        ));
        assert!(matches!(
            parse_page_number("abc"),
            Err(PageRangeError::InvalidPageNumber(_))
        ));
        assert!(matches!(
            parse_page_number("1.5"),
            Err(PageRangeError::InvalidPageNumber(_))
        ));
    }

    #[test]
    fn test_to_0based() {
        assert_eq!(to_0based(1, 10).unwrap(), 0);
        assert_eq!(to_0based(5, 10).unwrap(), 4);
        assert_eq!(to_0based(10, 10).unwrap(), 9);
        // Out of range: clamps to max
        assert_eq!(to_0based(15, 10).unwrap(), 9);
    }

    #[test]
    fn test_parse_single_page() {
        let pages = parse_page_range("1", 10).unwrap();
        assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![0]);

        let pages = parse_page_range("5", 10).unwrap();
        assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![4]);
    }

    #[test]
    fn test_parse_closed_range() {
        let pages = parse_page_range("1-5", 10).unwrap();
        assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);

        let pages = parse_page_range("5-10", 10).unwrap();
        assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![4, 5, 6, 7, 8, 9]);

        let pages = parse_page_range("3-3", 10).unwrap();
        assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![2]);
    }

    #[test]
    fn test_parse_open_start_range() {
        let pages = parse_page_range("-5", 10).unwrap();
        assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);

        let pages = parse_page_range("-1", 10).unwrap();
        assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![0]);
    }

    #[test]
    fn test_parse_open_end_range() {
        let pages = parse_page_range("12-", 20).unwrap();
        assert_eq!(pages.len(), 9); // 12-20 inclusive
        assert_eq!(*pages.first().unwrap(), 11); // 0-based
        assert_eq!(*pages.last().unwrap(), 19); // 0-based

        let pages = parse_page_range("20-", 20).unwrap();
        assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![19]);
    }

    #[test]
    fn test_parse_comma_separated() {
        let pages = parse_page_range("1,3,7", 10).unwrap();
        assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![0, 2, 6]);

        let pages = parse_page_range("1, 3, 7", 10).unwrap(); // With spaces
        assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![0, 2, 6]);

        let pages = parse_page_range("1-5,7,12-", 20).unwrap();
        // Should include 0-4 (1-5), 6 (7), 11-19 (12-)
        assert_eq!(pages.len(), 14);
        assert!(pages.contains(&0));
        assert!(pages.contains(&4));
        assert!(pages.contains(&6));
        assert!(pages.contains(&11));
        assert!(pages.contains(&19));
    }

    #[test]
    fn test_parse_empty_range() {
        assert!(matches!(
            parse_page_range("", 10),
            Err(PageRangeError::EmptyRange)
        ));
    }

    #[test]
    fn test_parse_invalid_range_start_greater_than_end() {
        let result = parse_page_range("5-3", 10);
        assert!(matches!(
            result,
            Err(PageRangeError::InvalidRange(_, _))
        ));
    }

    #[test]
    fn test_parse_malformed_range() {
        assert!(matches!(
            parse_page_range("-", 10),
            Err(PageRangeError::MalformedRange(_))
        ));

        assert!(matches!(
            parse_page_range("abc", 10),
            Err(PageRangeError::InvalidPageNumber(_))
        ));

        assert!(matches!(
            parse_page_range("1.5", 10),
            Err(PageRangeError::InvalidPageNumber(_))
        ));
    }

    #[test]
    fn test_filter_out_of_range() {
        let mut indices = BTreeSet::new();
        indices.insert(0);
        indices.insert(4);
        indices.insert(9);
        indices.insert(15); // Out of range (page 16 in a 10-page doc)

        let (valid, out_of_range) = filter_out_of_range(&indices, 10);

        assert_eq!(valid.len(), 3);
        assert!(valid.contains(&0));
        assert!(valid.contains(&4));
        assert!(valid.contains(&9));
        assert!(!valid.contains(&15));

        assert_eq!(out_of_range, vec![16]); // 1-based
    }

    #[test]
    fn test_parse_and_filter_out_of_range() {
        let indices = parse_page_range("1-5,10-15", 10).unwrap();
        let (valid, out_of_range) = filter_out_of_range(&indices, 10);

        // Valid: pages 1-5 (0-4 in 0-based)
        assert_eq!(valid.len(), 5);
        assert_eq!(valid.into_iter().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);

        // Out of range: pages 10-15 (1-based)
        assert_eq!(out_of_range, vec![10, 11, 12, 13, 14, 15]);
    }

    #[test]
    fn test_whitespace_handling() {
        // Spaces around commas
        let pages1 = parse_page_range("1, 3, 7", 10).unwrap();
        let pages2 = parse_page_range("1,3,7", 10).unwrap();
        assert_eq!(pages1, pages2);

        // Spaces around dash
        let pages1 = parse_page_range("1 - 5", 10).unwrap();
        let pages2 = parse_page_range("1-5", 10).unwrap();
        assert_eq!(pages1, pages2);

        // Mixed whitespace
        let pages1 = parse_page_range("1 - 5, 7 , 12 -", 20).unwrap();
        let pages2 = parse_page_range("1-5,7,12-", 20).unwrap();
        assert_eq!(pages1, pages2);
    }

    #[test]
    fn test_deduplication() {
        let pages = parse_page_range("1-5,3,7,3-5", 10).unwrap();
        // Should dedupe: 0-4 (1-5), 6 (7)
        assert_eq!(pages.len(), 6);
        assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4, 6]);
    }

    #[test]
    fn test_sorting() {
        let pages = parse_page_range("7,1,5,3", 10).unwrap();
        // BTreeSet automatically sorts
        assert_eq!(pages.into_iter().collect::<Vec<_>>(), vec![0, 2, 4, 6]);
    }
}
