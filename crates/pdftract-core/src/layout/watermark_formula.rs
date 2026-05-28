//! Watermark and Formula block classifiers (Phase 7 stubs).
//!
//! This module provides placeholder classifiers for Watermark and Formula block kinds.
//! Full detection implementation is deferred to Phase 7.
//!
//! ## Phase 7 Research Notes
//!
//! Watermark detection research: `docs/research/watermark-and-background-separation.md`
//! Formula detection research: See Phase 7.2 specification (math notation, OpenType Math tags)
//!
//! ## Current Implementation (Phase 4)
//!
//! - `classify_watermark`: Always returns `false` (no blocks classified as watermarks)
//! - `classify_formula`: Always returns `false` (no blocks classified as formulas)
//!
//! These stubs exist so that downstream consumers (JSON schema, markdown, profile
//! extraction) can be coded against the FULL taxonomy without breaking changes later.
//!
//! ## Phase 7 Implementation Plan
//!
//! ### Watermark Detection (Phase 7.1)
//!
//! TODO: Implement full watermark detection based on:
//! - Diagonal/rotated text spans
//! - Large font size spanning full page
//! - Low opacity or transparency
//! - Repeated content across pages (background patterns)
//! - Centered page-position text (e.g., "DRAFT", "CONFIDENTIAL")
//!
//! ### Formula Detection (Phase 7.2)
//!
//! TODO: Implement full formula detection based on:
//! - OpenType Math tags in PDF structure tree
//! - Monospace math fonts (e.g., Latin Modern Math)
//! - Mathematical notation patterns (symbols, operators)
//! - Adjacent to "Equation" captions
//!
//! See plan.md Phase 7.1 (watermark) and Phase 7.2 (formula) for full specifications.

use crate::layout::line::Block;

/// Classify a block as a watermark.
///
/// This is a Phase 4 stub that always returns `false`.
/// Full watermark detection is deferred to Phase 7.
///
/// # Arguments
///
/// * `_block` - The block to classify (unused in stub)
///
/// # Returns
///
/// Always returns `false` (no blocks classified as watermarks in Phase 4).
///
/// # Phase 7 Implementation
///
/// TODO: Implement watermark detection based on:
/// - Diagonal/rotated text (check LineMetadata rotation)
/// - Large font size (> 2x body median)
/// - Low opacity (check span alpha/transparency)
/// - Page-center positioning
/// - Cross-page repetition detection
///
/// See `docs/research/watermark-and-background-separation.md` for research notes.
pub fn classify_watermark<S>(_block: &Block<S>) -> bool {
    // Phase 4 stub: always return false
    // Phase 7 will implement full detection logic
    false
}

/// Classify a block as a formula/math block.
///
/// This is a Phase 4 stub that always returns `false`.
/// Full formula detection is deferred to Phase 7.
///
/// # Arguments
///
/// * `_block` - The block to classify (unused in stub)
///
/// # Returns
///
/// Always returns `false` (no blocks classified as formulas in Phase 4).
///
/// # Phase 7 Implementation
///
/// TODO: Implement formula detection based on:
/// - OpenType Math structure tags (from PDF StructTreeRoot)
/// - Math font detection (Latin Modern Math, STIX Math, etc.)
/// - Mathematical symbol patterns (∫, ∑, ∂, etc.)
/// - Adjacent to "Equation" or "Formula" captions
/// - Vertical stacking patterns (fractions, matrices)
///
/// See plan.md Phase 7.2 for full specification.
pub fn classify_formula<S>(_block: &Block<S>) -> bool {
    // Phase 4 stub: always return false
    // Phase 7 will implement full detection logic
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal test block structure.
    ///
    /// The actual `Block<S>` from line.rs requires a generic parameter for the
    /// span type. For stub testing, we only need to verify the functions exist
    /// and return false. The generic parameter is unused (the block is ignored),
    /// so we use `()` as a placeholder type.
    #[derive(Debug, Clone)]
    struct TestBlock {
        kind: String,
        text: String,
        bbox: [f32; 4],
        median_font_size: f32,
        column: usize,
    }

    /// Convert `TestBlock` to the `Block<()> type expected by the stubs.
    ///
    /// The stubs ignore their input, so we only need to satisfy the type system.
    /// The `Line<()> type requires `lines`, which we provide as an empty vec.
    fn make_test_block() -> crate::layout::line::Block<()> {
        crate::layout::line::Block {
            lines: vec![],
            kind: "test".to_string(),
            text: String::new(),
            bbox: [0.0, 0.0, 100.0, 20.0],
            median_font_size: 12.0,
            column: 0,
        }
    }

    #[test]
    fn test_classify_watermark_always_false() {
        let dummy_block = make_test_block();
        // Stub should always return false
        assert_eq!(classify_watermark(&dummy_block), false);
    }

    #[test]
    fn test_classify_formula_always_false() {
        let dummy_block = make_test_block();
        // Stub should always return false
        assert_eq!(classify_formula(&dummy_block), false);
    }

    #[test]
    fn test_watermark_stub_documentation() {
        // Verify the stub exists and compiles
        // This test documents the Phase 4 behavior
        let dummy_block = make_test_block();
        assert!(!classify_watermark(&dummy_block));
    }

    #[test]
    fn test_formula_stub_documentation() {
        // Verify the stub exists and compiles
        // This test documents the Phase 4 behavior
        let dummy_block = make_test_block();
        assert!(!classify_formula(&dummy_block));
    }
}
