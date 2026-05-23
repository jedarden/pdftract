//! Page classification for hybrid detection (Phase 5.1).
//!
//! This module implements per-page classification to determine the extraction
//! path: Vector (text-based), Scanned (image-based), Hybrid (mixed), or
//! BrokenVector (invisible text over scanned image).
//!
//! ## Hybrid Detection
//!
//! Hybrid detection uses an 8×8 grid decomposition. Each cell is classified
//! as vector, scanned, or mixed based on:
//! - **vector**: text_op_count > 0 AND char_validity > 0.6
//! - **scanned**: image_coverage > 0.80 AND text_op_count == 0
//! - **mixed**: neither condition met
//!
//! If ≥ 10 cells (≥ 15%) are vector AND ≥ 10 cells are scanned, the page
//! is classified as Hybrid. The set of scanned cell indexes is returned for
//! downstream OCR-only-on-cells routing in Phase 5.2.

use std::collections::BTreeSet;

/// Page classification result.
///
/// Represents the extraction path that should be used for this page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageClass {
    /// Vector (text-based) page - use Phase 3 content stream extraction.
    Vector,
    /// Scanned page - use Phase 5.2 raster extraction + OCR.
    Scanned,
    /// Hybrid page - use Phase 3 for vector cells + Phase 5.2 for scanned cells.
    Hybrid,
    /// BrokenVector (invisible text layer over scanned image).
    BrokenVector,
}

impl PageClass {
    /// Returns the JSON output string for this page type.
    ///
    /// Maps internal enum values to the schema's `page_type` field.
    pub fn as_type_str(&self) -> &'static str {
        match self {
            PageClass::Vector => "text",
            PageClass::Scanned => "scanned",
            PageClass::Hybrid => "mixed",
            PageClass::BrokenVector => "broken_vector",
        }
    }
}

/// Page classification result with confidence and metadata.
///
/// Contains the classification decision, confidence score, and optionally
/// the set of hybrid cell indexes for OCR routing.
#[derive(Debug, Clone)]
pub struct PageClassification {
    /// The classification decision.
    pub class: PageClass,
    /// Confidence score [0.0, 1.0].
    pub confidence: f32,
    /// For Hybrid pages: set of scanned cell indexes (row * 8 + col).
    /// None for non-Hybrid classifications.
    pub hybrid_cells: Option<BTreeSet<usize>>,
}

impl PageClassification {
    /// Create a new classification with the given class and confidence.
    pub fn new(class: PageClass, confidence: f32) -> Self {
        Self {
            class,
            confidence,
            hybrid_cells: None,
        }
    }

    /// Create a Hybrid classification with scanned cell indexes.
    pub fn hybrid(confidence: f32, hybrid_cells: BTreeSet<usize>) -> Self {
        Self {
            class: PageClass::Hybrid,
            confidence,
            hybrid_cells: Some(hybrid_cells),
        }
    }
}

/// Cell index in the 8×8 grid.
///
/// Cells are indexed as (row, col) where:
/// - row: 0..8 (0 = top of page in rendered orientation)
/// - col: 0..8 (0 = left of page)
///
/// The flat index is `row * 8 + col`, ranging from 0..63.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellIndex {
    /// Row index (0 = top, 7 = bottom).
    pub row: u8,
    /// Column index (0 = left, 7 = right).
    pub col: u8,
}

impl CellIndex {
    /// Create a new cell index.
    ///
    /// # Panics
    ///
    /// Panics if row or col >= 8.
    pub fn new(row: u8, col: u8) -> Self {
        assert!(row < 8, "row must be < 8");
        assert!(col < 8, "col must be < 8");
        Self { row, col }
    }

    /// Convert to flat index (0..63).
    #[inline]
    pub fn flat(&self) -> usize {
        (self.row as usize) * 8 + (self.col as usize)
    }

    /// Create from flat index (0..63).
    ///
    /// # Panics
    ///
    /// Panics if flat >= 64.
    pub fn from_flat(flat: usize) -> Self {
        assert!(flat < 64, "flat index must be < 64");
        Self {
            row: (flat / 8) as u8,
            col: (flat % 8) as u8,
        }
    }
}

/// Cell classification for a single grid cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellClass {
    /// Vector cell: has text operators with high character validity.
    Vector,
    /// Scanned cell: has high image coverage with no text operators.
    Scanned,
    /// Mixed cell: neither condition met (empty or ambiguous).
    Mixed,
}

/// Per-cell analysis data.
///
/// Contains the metrics computed for each grid cell during classification.
#[derive(Debug, Clone)]
pub struct CellData {
    /// Number of text operators in this cell.
    pub text_op_count: u32,
    /// Image coverage fraction [0.0, 1.0].
    pub image_coverage: f32,
    /// Character validity rate [0.0, 1.0] (fraction of valid Unicode chars).
    pub char_validity: f32,
}

impl CellData {
    /// Create new cell data with all zeros.
    pub fn empty() -> Self {
        Self {
            text_op_count: 0,
            image_coverage: 0.0,
            char_validity: 0.0,
        }
    }

    /// Classify this cell based on its metrics.
    pub fn classify(&self) -> CellClass {
        // Vector: has text operators AND high character validity
        if self.text_op_count > 0 && self.char_validity > 0.6 {
            return CellClass::Vector;
        }
        // Scanned: high image coverage AND no text operators
        if self.image_coverage > 0.80 && self.text_op_count == 0 {
            return CellClass::Scanned;
        }
        // Mixed: neither condition met (empty or ambiguous)
        CellClass::Mixed
    }
}

/// Grid-based page classifier.
///
/// Implements the 8×8 grid decomposition for hybrid detection.
pub struct GridClassifier {
    /// Page width in PDF user space units.
    width: f64,
    /// Page height in PDF user space units.
    height: f64,
    /// Page rotation in degrees (0, 90, 180, 270).
    rotation: i32,
    /// Cell data for each of the 64 cells.
    cells: [CellData; 64],
}

impl GridClassifier {
    /// Create a new grid classifier for a page.
    ///
    /// # Arguments
    ///
    /// * `width` - Page width in PDF user space units (after rotation applied).
    /// * `height` - Page height in PDF user space units (after rotation applied).
    /// * `rotation` - Page rotation in degrees (0, 90, 180, 270).
    pub fn new(width: f64, height: f64, rotation: i32) -> Self {
        Self {
            width,
            height,
            rotation,
            cells: std::array::from_fn(|_| CellData::empty()),
        }
    }

    /// Get mutable reference to cell data for a given cell index.
    pub fn cell_mut(&mut self, index: CellIndex) -> &mut CellData {
        &mut self.cells[index.flat()]
    }

    /// Get cell data for a given cell index.
    pub fn cell(&self, index: CellIndex) -> &CellData {
        &self.cells[index.flat()]
    }

    /// Compute which cell a point belongs to.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate in PDF user space.
    /// * `y` - Y coordinate in PDF user space.
    ///
    /// # Returns
    ///
    /// The cell index containing the point.
    ///
    /// # Note
    ///
    /// This method assumes the page has already been rotated to its
    /// rendered orientation. The MediaBox coordinates should be
    /// transformed by the rotation matrix before calling this method.
    pub fn point_to_cell(&self, x: f64, y: f64) -> CellIndex {
        // Clamp to page bounds
        let x_clamped = x.clamp(0.0, self.width);
        let y_clamped = y.clamp(0.0, self.height);

        // Compute cell indices
        // col 0 is at the left (x = 0), col 7 is at the right (x = width)
        let col_idx = ((x_clamped / self.width) * 8.0).floor() as u8;
        let col = col_idx.min(7);

        // row 0 is at the top (y = height), row 7 is at the bottom (y = 0)
        let y_ratio = y_clamped / self.height;
        let y_idx = (y_ratio * 8.0).floor() as u8;
        let y_idx_clamped = y_idx.min(7);
        let row = 7 - y_idx_clamped;

        CellIndex::new(row, col)
    }

    /// Classify the page based on cell analysis.
    ///
    /// Computes the final page classification by counting cell types
    /// and applying the hybrid detection rule (≥10 vector AND ≥10 scanned).
    ///
    /// # Returns
    ///
    /// A `PageClassification` containing the class, confidence, and
    /// optionally the set of scanned cell indexes for Hybrid pages.
    pub fn classify(&self) -> PageClassification {
        let mut vector_count = 0u32;
        let mut scanned_count = 0u32;
        let mut scanned_cells = BTreeSet::new();

        for (i, cell) in self.cells.iter().enumerate() {
            match cell.classify() {
                CellClass::Vector => vector_count += 1,
                CellClass::Scanned => {
                    scanned_count += 1;
                    scanned_cells.insert(i);
                }
                CellClass::Mixed => {}
            }
        }

        // Hybrid detection: ≥ 10 cells of each type (≥ 15% of 64)
        if vector_count >= 10 && scanned_count >= 10 {
            // Confidence is derived from the minimum of the two ratios
            let vector_ratio = vector_count as f32 / 64.0;
            let scanned_ratio = scanned_count as f32 / 64.0;
            let confidence = vector_ratio.min(scanned_ratio);

            return PageClassification::hybrid(confidence, scanned_cells);
        }

        // Non-hybrid classification based on dominant signal
        // This is a simplified version; the full Phase 5.1 includes
        // additional signals (no text ops, Tr=3, image coverage, etc.)
        if vector_count > scanned_count {
            PageClassification::new(PageClass::Vector, vector_count as f32 / 64.0)
        } else if scanned_count > 0 {
            PageClassification::new(PageClass::Scanned, scanned_count as f32 / 64.0)
        } else {
            // Empty page (no vector, no scanned) - default to Vector
            // with low confidence; will be handled by other signals
            // in the full classifier
            PageClassification::new(PageClass::Vector, 0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_index_flat_conversion() {
        let cell = CellIndex::new(0, 0);
        assert_eq!(cell.flat(), 0);

        let cell = CellIndex::new(0, 1);
        assert_eq!(cell.flat(), 1);

        let cell = CellIndex::new(1, 0);
        assert_eq!(cell.flat(), 8);

        let cell = CellIndex::new(7, 7);
        assert_eq!(cell.flat(), 63);

        let cell = CellIndex::from_flat(0);
        assert_eq!(cell.row, 0);
        assert_eq!(cell.col, 0);

        let cell = CellIndex::from_flat(8);
        assert_eq!(cell.row, 1);
        assert_eq!(cell.col, 0);

        let cell = CellIndex::from_flat(63);
        assert_eq!(cell.row, 7);
        assert_eq!(cell.col, 7);
    }

    #[test]
    fn test_cell_data_classify_vector() {
        let cell = CellData {
            text_op_count: 10,
            image_coverage: 0.1,
            char_validity: 0.9,
        };
        assert_eq!(cell.classify(), CellClass::Vector);
    }

    #[test]
    fn test_cell_data_classify_scanned() {
        let cell = CellData {
            text_op_count: 0,
            image_coverage: 0.9,
            char_validity: 0.0,
        };
        assert_eq!(cell.classify(), CellClass::Scanned);
    }

    #[test]
    fn test_cell_data_classify_mixed() {
        // Empty cell
        let cell = CellData {
            text_op_count: 0,
            image_coverage: 0.0,
            char_validity: 0.0,
        };
        assert_eq!(cell.classify(), CellClass::Mixed);

        // Text but low validity (char_validity <= 0.6)
        let cell = CellData {
            text_op_count: 10,
            image_coverage: 0.1,
            char_validity: 0.5,
        };
        assert_eq!(cell.classify(), CellClass::Mixed);

        // Image but also text with low validity
        let cell = CellData {
            text_op_count: 1,
            image_coverage: 0.9,
            char_validity: 0.5,
        };
        assert_eq!(cell.classify(), CellClass::Mixed);

        // Image with low coverage (< 0.80)
        let cell = CellData {
            text_op_count: 0,
            image_coverage: 0.5,
            char_validity: 0.0,
        };
        assert_eq!(cell.classify(), CellClass::Mixed);
    }

    #[test]
    fn test_grid_classifier_point_to_cell() {
        let classifier = GridClassifier::new(612.0, 792.0, 0);

        // Bottom-left corner -> row 7, col 0
        let cell = classifier.point_to_cell(0.0, 0.0);
        assert_eq!(cell.row, 7);
        assert_eq!(cell.col, 0);

        // Top-left corner -> row 0, col 0
        let cell = classifier.point_to_cell(0.0, 792.0);
        assert_eq!(cell.row, 0);
        assert_eq!(cell.col, 0);

        // Top-right corner -> row 0, col 7
        let cell = classifier.point_to_cell(612.0, 792.0);
        assert_eq!(cell.row, 0);
        assert_eq!(cell.col, 7);

        // Bottom-right corner -> row 7, col 7
        let cell = classifier.point_to_cell(612.0, 0.0);
        assert_eq!(cell.row, 7);
        assert_eq!(cell.col, 7);

        // Center -> row 3-4, col 3-4
        let cell = classifier.point_to_cell(306.0, 396.0);
        assert!(cell.row >= 3 && cell.row <= 4);
        assert!(cell.col >= 3 && cell.col <= 4);
    }

    #[test]
    fn test_grid_classifier_hybrid_detection() {
        let mut classifier = GridClassifier::new(612.0, 792.0, 0);

        // Set up a hybrid page: top 2 rows (16 cells) are vector,
        // bottom 6 rows (48 cells) are scanned
        for row in 0..8 {
            for col in 0..8 {
                let idx = CellIndex::new(row, col);
                let cell = classifier.cell_mut(idx);
                if row < 2 {
                    // Top rows: vector
                    cell.text_op_count = 10;
                    cell.char_validity = 0.95;
                    cell.image_coverage = 0.1;
                } else {
                    // Bottom rows: scanned
                    cell.text_op_count = 0;
                    cell.image_coverage = 0.9;
                    cell.char_validity = 0.0;
                }
            }
        }

        let result = classifier.classify();
        assert_eq!(result.class, PageClass::Hybrid);
        assert!(result.hybrid_cells.is_some());
        assert_eq!(result.hybrid_cells.as_ref().unwrap().len(), 48);

        // Verify scanned cells are from rows 2-7 only
        for flat in result.hybrid_cells.as_ref().unwrap() {
            let cell = CellIndex::from_flat(*flat);
            assert!(cell.row >= 2, "scanned cell should be in rows 2-7");
        }
    }

    #[test]
    fn test_grid_classifier_below_threshold() {
        let mut classifier = GridClassifier::new(612.0, 792.0, 0);

        // Set up a page with 9 vector cells and 9 scanned cells
        // (just below the 10-cell threshold)
        // Use a 3x3 arrangement for each type
        for row in 0..3 {
            for col in 0..3 {
                let vector_cell = classifier.cell_mut(CellIndex::new(row, col));
                vector_cell.text_op_count = 10;
                vector_cell.char_validity = 0.95;
                vector_cell.image_coverage = 0.1;
            }
        }
        for row in 5..8 {
            for col in 5..8 {
                let scanned_cell = classifier.cell_mut(CellIndex::new(row, col));
                scanned_cell.text_op_count = 0;
                scanned_cell.image_coverage = 0.9;
                scanned_cell.char_validity = 0.0;
            }
        }

        let result = classifier.classify();
        // Should NOT be Hybrid (below threshold)
        assert_ne!(result.class, PageClass::Hybrid);
        assert!(result.hybrid_cells.is_none());
    }

    #[test]
    fn test_page_class_as_type_str() {
        assert_eq!(PageClass::Vector.as_type_str(), "text");
        assert_eq!(PageClass::Scanned.as_type_str(), "scanned");
        assert_eq!(PageClass::Hybrid.as_type_str(), "mixed");
        assert_eq!(PageClass::BrokenVector.as_type_str(), "broken_vector");
    }

    #[test]
    fn test_page_classification_hybrid() {
        let mut cells = BTreeSet::new();
        cells.insert(16);
        cells.insert(17);

        let classification = PageClassification::hybrid(0.75, cells);

        assert_eq!(classification.class, PageClass::Hybrid);
        assert_eq!(classification.confidence, 0.75);
        assert!(classification.hybrid_cells.is_some());
        assert_eq!(classification.hybrid_cells.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_determinism_btree_set() {
        // Verify BTreeSet produces deterministic iteration order
        let mut set1 = BTreeSet::new();
        set1.insert(5);
        set1.insert(2);
        set1.insert(8);

        let mut set2 = BTreeSet::new();
        set2.insert(8);
        set2.insert(5);
        set2.insert(2);

        // Iteration order should be the same
        assert_eq!(set1.iter().collect::<Vec<_>>(), set2.iter().collect::<Vec<_>>());
    }

    #[test]
    #[should_panic(expected = "row must be < 8")]
    fn test_cell_index_invalid_row() {
        CellIndex::new(8, 0);
    }

    #[test]
    #[should_panic(expected = "col must be < 8")]
    fn test_cell_index_invalid_col() {
        CellIndex::new(0, 8);
    }

    #[test]
    #[should_panic(expected = "flat index must be < 64")]
    fn test_cell_index_invalid_flat() {
        CellIndex::from_flat(64);
    }

    #[test]
    fn test_critical_hybrid_page_text_header_scanned_body() {
        // Critical test from bead pdftract-347:
        // Hybrid page with text header (top 2 rows) + scanned body (bottom 6 rows)
        // -> Hybrid with hybrid_cells containing rows 2-7 only
        let mut classifier = GridClassifier::new(612.0, 792.0, 0);

        // Top 2 rows: vector (text header)
        for row in 0..2 {
            for col in 0..8 {
                let idx = CellIndex::new(row, col);
                let cell = classifier.cell_mut(idx);
                cell.text_op_count = 15;
                cell.char_validity = 0.95;
                cell.image_coverage = 0.05;
            }
        }

        // Bottom 6 rows: scanned (body)
        for row in 2..8 {
            for col in 0..8 {
                let idx = CellIndex::new(row, col);
                let cell = classifier.cell_mut(idx);
                cell.text_op_count = 0;
                cell.image_coverage = 0.90;
                cell.char_validity = 0.0;
            }
        }

        let result = classifier.classify();

        // Should be classified as Hybrid
        assert_eq!(result.class, PageClass::Hybrid);
        assert!(result.hybrid_cells.is_some());

        let scanned_cells = result.hybrid_cells.as_ref().unwrap();
        assert_eq!(scanned_cells.len(), 48); // 6 rows * 8 cols

        // Verify all scanned cells are from rows 2-7 only
        for flat in scanned_cells {
            let cell = CellIndex::from_flat(*flat);
            assert!(cell.row >= 2 && cell.row <= 7,
                "scanned cell at flat {} should be in rows 2-7, got row {}",
                flat, cell.row);
        }
    }

    #[test]
    fn test_determinism_classify_twice() {
        // Verify that classifying the same page twice produces byte-identical
        // hybrid_cells serialization (BTreeSet ensures deterministic ordering)
        let mut classifier1 = GridClassifier::new(612.0, 792.0, 0);
        let mut classifier2 = GridClassifier::new(612.0, 792.0, 0);

        // Set up identical hybrid pages
        for row in 0..8 {
            for col in 0..8 {
                let is_scanned = row >= 4 && col >= 4;
                let cell1 = classifier1.cell_mut(CellIndex::new(row, col));
                let cell2 = classifier2.cell_mut(CellIndex::new(row, col));

                if is_scanned {
                    cell1.text_op_count = 0;
                    cell1.image_coverage = 0.9;
                    cell1.char_validity = 0.0;

                    cell2.text_op_count = 0;
                    cell2.image_coverage = 0.9;
                    cell2.char_validity = 0.0;
                } else {
                    cell1.text_op_count = 10;
                    cell1.char_validity = 0.95;
                    cell1.image_coverage = 0.1;

                    cell2.text_op_count = 10;
                    cell2.char_validity = 0.95;
                    cell2.image_coverage = 0.1;
                }
            }
        }

        let result1 = classifier1.classify();
        let result2 = classifier2.classify();

        assert_eq!(result1.class, result2.class);
        assert_eq!(result1.confidence, result2.confidence);

        // Verify hybrid_cells serialize identically
        let json1 = serde_json::to_string(&result1.hybrid_cells).unwrap();
        let json2 = serde_json::to_string(&result2.hybrid_cells).unwrap();
        assert_eq!(json1, json2);
    }

    #[test]
    fn test_exactly_10_cells_threshold() {
        // Test the exact threshold: 10 vector cells + 10 scanned cells = Hybrid
        let mut classifier = GridClassifier::new(612.0, 792.0, 0);

        // 10 vector cells (row 0, cols 0-7 + row 1, cols 0-1)
        for col in 0..8 {
            let cell = classifier.cell_mut(CellIndex::new(0, col));
            cell.text_op_count = 10;
            cell.char_validity = 0.95;
            cell.image_coverage = 0.1;
        }
        for col in 0..2 {
            let cell = classifier.cell_mut(CellIndex::new(1, col));
            cell.text_op_count = 10;
            cell.char_validity = 0.95;
            cell.image_coverage = 0.1;
        }

        // 10 scanned cells (row 7, cols 0-7 + row 6, cols 0-1)
        for col in 0..8 {
            let cell = classifier.cell_mut(CellIndex::new(7, col));
            cell.text_op_count = 0;
            cell.image_coverage = 0.9;
            cell.char_validity = 0.0;
        }
        for col in 0..2 {
            let cell = classifier.cell_mut(CellIndex::new(6, col));
            cell.text_op_count = 0;
            cell.image_coverage = 0.9;
            cell.char_validity = 0.0;
        }

        let result = classifier.classify();
        assert_eq!(result.class, PageClass::Hybrid);
    }

    #[test]
    fn test_rotation_handling() {
        // Verify that rotation is stored (actual rotation handling
        // requires transforming coordinates before calling point_to_cell)
        let classifier_rotated = GridClassifier::new(792.0, 612.0, 90);
        assert_eq!(classifier_rotated.rotation, 90);
        assert_eq!(classifier_rotated.width, 792.0);
        assert_eq!(classifier_rotated.height, 612.0);

        // After 90-degree rotation, width and height are swapped
        let classifier_normal = GridClassifier::new(612.0, 792.0, 0);
        assert_eq!(classifier_normal.rotation, 0);
        assert_eq!(classifier_normal.width, 612.0);
        assert_eq!(classifier_normal.height, 792.0);
    }

    #[test]
    fn test_empty_page_classification() {
        // Empty page (no text, no images) should default to Vector with low confidence
        let classifier = GridClassifier::new(612.0, 792.0, 0);
        let result = classifier.classify();

        // Empty pages default to Vector (will be overridden by other signals in full classifier)
        assert_eq!(result.class, PageClass::Vector);
        assert_eq!(result.confidence, 0.0);
        assert!(result.hybrid_cells.is_none());
    }
}
