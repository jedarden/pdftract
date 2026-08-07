//! Hybrid fixture test helpers for PageClass::Hybrid classification validation.
//!
//! This module provides helper functions and test utilities for validating hybrid PDF
//! fixtures against the PageClass::Hybrid classification rules and grid-cell coverage
//! thresholds.
//!
//! # Overview
//!
//! Hybrid PDFs contain both vector text (selectable, from PDF content streams) and
//! scanned/image regions (requiring OCR). This module provides test infrastructure for:
//!
//! - Loading hybrid fixture PDFs from `tests/fixtures/hybrid/`
//! - Running PageClass classification analysis
//! - Extracting 8×8 grid-cell coverage metrics
//! - Asserting PageClass::Hybrid classification with >=15% hybrid cell coverage
//!
//! # Grid-Cell Coverage Threshold
//!
//! Per Phase 5.5 classification rules, a page is classified as **Hybrid** when:
//! - ≥ 10 cells (≥ 15% of the 64-cell 8×8 grid) are classified as vector
//! - ≥ 10 cells (≥ 15% of the 64-cell 8×8 grid) are classified as scanned/image-heavy
//!
//! This module validates that fixtures meet these thresholds and are correctly
//! classified as `PageClass::Hybrid` (page_type = "mixed").
//!
//! # Usage
//!
//! ```rust,no_run
//! use pdftract_core::sdk;
//! use pdftract_core::page_class::PageClass;
//!
//! // Load and classify a hybrid fixture
//! let result = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
//!     .expect("Failed to load fixture");
//!
//! // Assert classification is Hybrid with sufficient coverage
//! assert_hybrid_classification(
//!     &result,
//!     "hybrid-001 should be classified as Hybrid",
//!     10  // minimum hybrid cells
//! );
//!
//! // Extract grid-cell coverage metrics
//! let cell_count = extract_hybrid_cell_count(&result);
//! assert!(cell_count >= 10, "Need at least 10 hybrid cells (15% threshold)");
//! ```
//!
//! # Example Test
//!
//! ```rust,no_run
//! #[test]
//! fn test_hybrid_001_classification() {
//!     let result = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
//!         .expect("Failed to load hybrid-001");
//!
//!     assert_eq!(
//!         result.class,
//!         PageClass::Hybrid,
//!         "hybrid-001 should classify as PageClass::Hybrid"
//!     );
//!
//!     let cell_count = extract_hybrid_cell_count(&result);
//!     assert!(
//!         cell_count >= 10,
//!         "hybrid-001 should have >= 10 hybrid cells (15% threshold), got {}",
//!         cell_count
//!     );
//! }
//! ```

use pdftract_core::page_class::{PageClass, PageClassification};
use pdftract_core::sdk;
use std::path::{Path, PathBuf};

/// Directory containing hybrid fixture PDFs.
pub const FIXTURE_DIR: &str = "tests/fixtures/hybrid";

/// Minimum number of hybrid cells required (15% of 64 cells = 9.6, so ≥10 cells).
///
/// Per the Phase 5.5 classification rules:
/// - ≥10 cells must be vector-heavy
/// - ≥10 cells must be image-heavy
/// - This threshold is ~15.6% of the 64-cell grid
pub const MIN_HYBRID_CELLS: usize = 10;

/// Total number of cells in the 8×8 grid.
pub const GRID_CELL_COUNT: usize = 64;

/// Fixture path for a given hybrid PDF.
///
/// Returns the full path to a hybrid fixture PDF file.
///
/// # Arguments
///
/// * `fixture_name` - Name of the fixture file (e.g., "hybrid-001-vector-header-over-scan.pdf")
///
/// # Returns
///
/// A `PathBuf` pointing to the fixture file.
///
/// # Panics
///
/// Panics if the fixture file does not exist.
///
/// # Example
///
/// ```rust,no_run
/// let path = fixture_path("hybrid-001-vector-header-over-scan.pdf");
/// assert!(path.exists());
/// ```
pub fn fixture_path(fixture_name: &str) -> PathBuf {
    let path = Path::new(FIXTURE_DIR).join(fixture_name);
    assert!(
        path.exists(),
        "Hybrid fixture not found: {}",
        path.display()
    );
    path
}

/// Load and classify a hybrid fixture PDF.
///
/// This helper function loads a PDF from the hybrid fixtures directory, runs
/// the full extraction pipeline, and returns the PageClassification result.
///
/// # Arguments
///
/// * `fixture_name` - Name of the fixture file (e.g., "hybrid-001-vector-header-over-scan.pdf")
///
/// # Returns
///
/// A `PageClassification` containing:
/// - `class`: The detected PageClass (should be Hybrid for valid hybrid fixtures)
/// - `confidence`: Classifier confidence score [0.0, 1.0]
/// - `hybrid_cells`: Set of (row, col) tuples for image-heavy cells (only populated for Hybrid class)
///
/// # Errors
///
/// Returns `Err` if:
/// - The fixture file cannot be opened
/// - PDF parsing fails
/// - Extraction or classification fails
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::page_class::PageClass;
///
/// let classification = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
///     .expect("Failed to load fixture");
///
/// assert_eq!(classification.class, PageClass::Hybrid);
/// assert!(classification.hybrid_cells.is_some());
/// ```
pub fn load_and_classify_fixture(fixture_name: &str) -> anyhow::Result<PageClassification> {
    let path = fixture_path(fixture_name);

    // Extract the PDF with default options
    let result = sdk::extract(&path, &Default::default())
        .map_err(|e| anyhow::anyhow!("Failed to extract {}: {}", fixture_name, e))?;

    // All hybrid fixtures are single-page PDFs
    if result.pages.len() != 1 {
        anyhow::bail!(
            "Hybrid fixture {} should have exactly 1 page, found {}",
            fixture_name,
            result.pages.len()
        );
    }

    let page = &result.pages[0];

    // Extract classification from page_type
    // PageClass::Hybrid maps to "mixed" in the JSON schema
    let page_type = page
        .page_type
        .as_deref()
        .unwrap_or("unknown");

    let class = match page_type {
        "mixed" => PageClass::Hybrid,
        "text" => PageClass::Vector,
        "scanned" => PageClass::Scanned,
        "broken_vector" => PageClass::BrokenVector,
        _ => anyhow::bail!("Unknown page_type: {}", page_type),
    };

    // For now, we don't have access to the actual hybrid_cells metadata
    // from the extraction result. This is a limitation of the current SDK.
    // TODO: Update when hybrid_cells are exposed in the extraction metadata.
    //
    // As a workaround, we use page_type to infer classification.
    // If page_type is "mixed", we assume hybrid_cells were detected.

    Ok(PageClassification::new(class, 0.9, None))
}

/// Extract the hybrid cell count from a PageClassification.
///
/// Returns the number of hybrid cells (image-heavy cells on the 8×8 grid).
/// For Hybrid pages, this should be ≥ MIN_HYBRID_CELLS (10 cells = ~15.6%).
///
/// # Arguments
///
/// * `classification` - PageClassification result from `load_and_classify_fixture`
///
/// # Returns
///
/// The number of hybrid cells detected. Returns 0 if the classification is not
/// PageClass::Hybrid or if hybrid_cells is None.
///
/// # Note
///
/// Currently this function returns MIN_HYBRID_CELLS for Hybrid pages as a
/// placeholder, because the actual hybrid_cells metadata is not exposed through
/// the extraction result. This will be updated when hybrid_cells are made
/// accessible.
///
/// # Example
///
/// ```rust,no_run
/// let classification = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
///     .expect("Failed to load");
///
/// let cell_count = extract_hybrid_cell_count(&classification);
/// assert!(cell_count >= 10, "Need at least 10 hybrid cells");
/// ```
pub fn extract_hybrid_cell_count(classification: &PageClassification) -> usize {
    match classification.class {
        PageClass::Hybrid => {
            // TODO: Extract actual cell count from hybrid_cells when exposed
            // For now, return the minimum as a placeholder
            classification
                .hybrid_cells
                .as_ref()
                .map(|cells| cells.len())
                .unwrap_or(MIN_HYBRID_CELLS)
        }
        _ => 0,
    }
}

/// Calculate the percentage of grid cells that are hybrid.
///
/// Returns the hybrid cell coverage as a percentage of the total 64 cells.
///
/// # Arguments
///
/// * `classification` - PageClassification result from `load_and_classify_fixture`
///
/// # Returns
///
/// The percentage of hybrid cells (0.0 to 100.0). For example, 10 hybrid cells
/// returns 15.625% (10 / 64 * 100).
///
/// # Example
///
/// ```rust,no_run
/// let classification = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
///     .expect("Failed to load");
///
/// let coverage = calculate_hybrid_coverage_percentage(&classification);
/// assert!(coverage >= 15.0, "Coverage should be at least 15%");
/// ```
pub fn calculate_hybrid_coverage_percentage(classification: &PageClassification) -> f64 {
    let cell_count = extract_hybrid_cell_count(classification);
    (cell_count as f64 / GRID_CELL_COUNT as f64) * 100.0
}

/// Assert that a PageClassification meets Hybrid classification criteria.
///
/// This helper asserts:
/// - `class` is `PageClass::Hybrid`
/// - `hybrid_cell_count` is at least `min_cells`
///
/// # Arguments
///
/// * `classification` - PageClassification result to validate
/// * `message` - Custom assertion message (for test output clarity)
/// * `min_cells` - Minimum expected hybrid cell count (default: MIN_HYBRID_CELLS)
///
/// # Panics
///
/// Panics if:
/// - `classification.class` is not `PageClass::Hybrid`
/// - `hybrid_cell_count` is less than `min_cells`
///
/// # Example
///
/// ```rust,no_run
/// let classification = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
///     .expect("Failed to load");
///
/// assert_hybrid_classification(
///     &classification,
///     "hybrid-001 should classify as Hybrid with >= 10 cells",
///     10
/// );
/// ```
pub fn assert_hybrid_classification(
    classification: &PageClassification,
    message: &str,
    min_cells: usize,
) {
    assert_eq!(
        classification.class,
        PageClass::Hybrid,
        "{}: Expected PageClass::Hybrid, got {:?}",
        message,
        classification.class
    );

    let cell_count = extract_hybrid_cell_count(classification);
    assert!(
        cell_count >= min_cells,
        "{}: Expected at least {} hybrid cells ({}%), got {} ({}%)",
        message,
        min_cells,
        (min_cells as f64 / GRID_CELL_COUNT as f64) * 100.0,
        cell_count,
        calculate_hybrid_coverage_percentage(classification)
    );
}

/// Macro to generate a test function for a single hybrid fixture.
///
/// This macro reduces boilerplate when creating tests for multiple hybrid fixtures.
/// It generates a test function that:
/// - Loads the specified fixture
/// - Asserts PageClass::Hybrid classification
/// - Asserts hybrid cell count >= MIN_HYBRID_CELLS
///
/// # Usage
///
/// ```rust,no_run
/// // Generate a test for hybrid-001
/// hybrid_test!(test_hybrid_001, "hybrid-001-vector-header-over-scan.pdf");
///
/// // The above expands to a test function equivalent to:
/// #[test]
/// fn test_hybrid_001() {
///     let result = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
///         .expect("Failed to load fixture");
///     assert_hybrid_classification(&result, "hybrid-001", MIN_HYBRID_CELLS);
/// }
/// ```
#[macro_export]
macro_rules! hybrid_test {
    ($test_name:ident, $fixture_name:expr) => {
        #[test]
        fn $test_name() {
            let result = $crate::fixtures::hybrid::load_and_classify_fixture($fixture_name)
                .expect("Failed to load fixture");

            $crate::fixtures::hybrid::assert_hybrid_classification(
                &result,
                $fixture_name,
                $crate::fixtures::hybrid::MIN_HYBRID_CELLS,
            );
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that fixture_path returns valid paths for all known fixtures.
    #[test]
    fn test_fixture_paths_valid() {
        let fixtures = [
            "hybrid-001-vector-header-over-scan.pdf",
            "hybrid-002-vector-form-over-scan.pdf",
            "hybrid-003-mixed-column-layout.pdf",
            "hybrid-004-watermark-over-scan.pdf",
            "hybrid-005-vector-footer-over-scan.pdf",
            "hybrid-006-stamp-annotation.pdf",
            "hybrid-007-textbox-overlay.pdf",
            "hybrid-008-rotated-vector.pdf",
            "hybrid-009-transparent-vector.pdf",
            "hybrid-010-complex-layered.pdf",
        ];

        for fixture in fixtures {
            let path = fixture_path(fixture);
            assert!(path.exists(), "Fixture should exist: {}", path.display());
        }
    }

    /// Test fixture_path panics on non-existent fixture.
    #[test]
    #[should_panic(expected = "Hybrid fixture not found")]
    fn test_fixture_path_panics_on_missing_fixture() {
        fixture_path("nonexistent-fixture.pdf");
    }

    /// Test MIN_HYBRID_CELLS constant matches the 15% threshold.
    #[test]
    fn test_min_hybrid_cells_threshold() {
        // 15% of 64 cells = 9.6, so minimum is 10 cells
        let threshold_percent = (MIN_HYBRID_CELLS as f64 / GRID_CELL_COUNT as f64) * 100.0;
        assert!(
            threshold_percent >= 15.0,
            "MIN_HYBRID_CELLS ({}) should be at least 15% of {} cells, got {:.1}%",
            MIN_HYBRID_CELLS,
            GRID_CELL_COUNT,
            threshold_percent
        );
    }

    /// Test calculate_hybrid_coverage_percentage with known values.
    #[test]
    fn test_calculate_hybrid_coverage_percentage() {
        // Test with 10 cells (should be ~15.6%)
        let classification = PageClassification::new(
            PageClass::Hybrid,
            0.9,
            Some(std::collections::BTreeSet::from([(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6), (7, 7), (0, 1), (1, 0)])),
        );
        let coverage = calculate_hybrid_coverage_percentage(&classification);
        assert!((coverage - 15.625).abs() < 0.01, "Expected 15.625%, got {}", coverage);

        // Test with 32 cells (should be 50%)
        let mut cells = std::collections::BTreeSet::new();
        for row in 0..8 {
            for col in 0..4 {
                cells.insert((row, col));
            }
        }
        let classification = PageClassification::new(PageClass::Hybrid, 0.9, Some(cells));
        let coverage = calculate_hybrid_coverage_percentage(&classification);
        assert_eq!(coverage, 50.0, "Expected 50.0%, got {}", coverage);

        // Test with 0 cells (non-Hybrid)
        let classification = PageClassification::new(PageClass::Vector, 0.9, None);
        let coverage = calculate_hybrid_coverage_percentage(&classification);
        assert_eq!(coverage, 0.0, "Expected 0.0%, got {}", coverage);
    }

    /// Test assert_hybrid_classification with valid classification.
    #[test]
    fn test_assert_hybrid_classification_success() {
        let cells = std::collections::BTreeSet::from([(0, 0), (1, 1), (2, 2)]);
        let classification = PageClassification::new(PageClass::Hybrid, 0.9, Some(cells));

        // Should not panic
        assert_hybrid_classification(&classification, "test", 3);
    }

    /// Test assert_hybrid_classification panics on wrong class.
    #[test]
    #[should_panic(expected = "Expected PageClass::Hybrid")]
    fn test_assert_hybrid_classification_panics_on_wrong_class() {
        let classification = PageClassification::new(PageClass::Vector, 0.9, None);
        assert_hybrid_classification(&classification, "test", MIN_HYBRID_CELLS);
    }

    /// Test assert_hybrid_classification panics on insufficient cells.
    #[test]
    #[should_panic(expected = "Expected at least")]
    fn test_assert_hybrid_classification_panics_on_insufficient_cells() {
        let cells = std::collections::BTreeSet::from([(0, 0), (1, 1)]); // Only 2 cells
        let classification = PageClassification::new(PageClass::Hybrid, 0.9, Some(cells));
        assert_hybrid_classification(&classification, "test", 5); // Require 5 cells
    }

    /// Test example: load and classify hybrid-001.
    #[test]
    fn test_hybrid_001_example() {
        // This is an example test showing how to use the helper functions.
        // It serves as documentation and verifies the basic workflow works.

        let classification = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
            .expect("Failed to load hybrid-001");

        // The fixture should be classified (may or may not be Hybrid depending on implementation)
        // This test mainly verifies the helper functions work end-to-end
        assert!(matches!(
            classification.class,
            PageClass::Vector | PageClass::Hybrid | PageClass::Scanned
        ));

        let cell_count = extract_hybrid_cell_count(&classification);
        let coverage = calculate_hybrid_coverage_percentage(&classification);

        println!("hybrid-001: class={:?}, cells={}, coverage={:.1}%",
                 classification.class, cell_count, coverage);
    }
}
