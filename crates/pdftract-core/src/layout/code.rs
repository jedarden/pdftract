//! Code block classifier (Phase 4).
//!
//! This module implements classification of blocks as code based on:
//! 1. All spans use a monospace font
//! 2. The block is indented ≥ 2em relative to the surrounding body text
//!
//! Code blocks are typically distinguished by:
//! - Monospace font (Courier, Monaco, Consolas, etc.)
//! - Indentation from the main text column
//! - Consistent font throughout the block

use crate::font::strip_subset_prefix;

/// Check if a font name indicates a monospace font.
///
/// A font is considered monospace if its name (with subset prefix stripped)
/// contains any of the following case-insensitive substrings:
/// - "Mono"
/// - "Courier"
/// - "Code"
/// - "Fixed"
/// - "Console"
///
/// # Arguments
///
/// * `font_name` - The font name from the PDF (may include subset prefix)
///
/// # Returns
///
/// `true` if the font name indicates a monospace font, `false` otherwise.
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::code::is_monospace_font_name;
///
/// assert!(is_monospace_font_name("Courier"));
/// assert!(is_monospace_font_name("Courier-New"));
/// assert!(is_monospace_font_name("Mono"));
/// assert!(is_monospace_font_name("SourceCodePro"));
/// assert!(is_monospace_font_name("Consolas"));
/// assert!(is_monospace_font_name("ABCDEF+Courier")); // Subset prefix
///
/// assert!(!is_monospace_font_name("Times-Roman"));
/// assert!(!is_monospace_font_name("Helvetica"));
/// ```
pub fn is_monospace_font_name(font_name: &str) -> bool {
    let stripped = strip_subset_prefix(font_name).to_lowercase();

    let monospace_indicators = ["mono", "courier", "code", "fixed", "console"];

    monospace_indicators
        .iter()
        .any(|&indicator| stripped.contains(indicator))
}

/// Check if the FixedPitch flag (bit 0) is set in font descriptor flags.
///
/// PDF font descriptor flags use bit 0 to indicate fixed-pitch (monospace) fonts.
///
/// # Arguments
///
/// * `flags` - Optional flags value from the FontDescriptor
///
/// # Returns
///
/// `true` if the FixedPitch flag (bit 0) is set, `false` otherwise.
/// Returns `false` if flags is None.
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::code::is_fixed_pitch_flag;
///
/// assert!(is_fixed_pitch_flag(Some(1))); // Bit 0 set
/// assert!(is_fixed_pitch_flag(Some(0b00000001)));
/// assert!(!is_fixed_pitch_flag(Some(0))); // Bit 0 not set
/// assert!(!is_fixed_pitch_flag(Some(2))); // Bit 1 set, not bit 0
/// assert!(!is_fixed_pitch_flag(None)); // No flags
/// ```
pub fn is_fixed_pitch_flag(flags: Option<u32>) -> bool {
    match flags {
        Some(f) => (f & 0x1) == 1,
        None => false,
    }
}

/// Check if a span uses a monospace font.
///
/// A span is considered monospace if EITHER:
/// 1. The font name indicates monospace (via `is_monospace_font_name`)
/// 2. The FixedPitch flag is set in the font descriptor
///
/// # Arguments
///
/// * `font_name` - The font name from the PDF
/// * `flags` - Optional flags value from the FontDescriptor
///
/// # Returns
///
/// `true` if the span uses a monospace font, `false` otherwise.
pub fn is_monospace_span(font_name: &str, flags: Option<u32>) -> bool {
    is_monospace_font_name(font_name) || is_fixed_pitch_flag(flags)
}

/// Classify a block as code based on monospace and indentation criteria.
///
/// A block is classified as code if ALL of the following are true:
/// 1. All spans in the block use a monospace font
/// 2. The block is indented ≥ 2em relative to the column baseline
///
/// # Arguments
///
/// * `block` - The block to classify
/// * `column_baseline_x0` - The median x0 of non-code paragraph blocks in the column
/// * `font_size` - The font size in points (used to compute em width)
///
/// # Returns
///
/// `true` if the block should be classified as code, `false` otherwise.
///
/// # Font Information
///
/// This function assumes that the block's spans have font information
/// accessible via a `font()` method that returns the font name, and
/// optionally a `flags()` method for FontDescriptor flags.
///
/// # Indentation Calculation
///
/// The indentation threshold is 2em, where:
/// - em_width = font_size (in points)
/// - threshold = 2.0 * font_size
///
/// A block is considered indented if its x0 position is at least
/// `column_baseline_x0 + 2 * font_size` points from the left.
///
/// # Examples
///
/// ```ignore
/// use pdftract_core::layout::code::classify_code;
///
/// // All-Courier block indented 24pt with font_size 12pt (2em=24pt)
/// let is_code = classify_code(&block, 72.0, 12.0);
/// assert!(is_code);
///
/// // Monospace block not indented
/// let is_code = classify_code(&block, 100.0, 12.0);
/// assert!(!is_code);
/// ```
pub fn classify_code<S>(
    block: &crate::layout::line::Block<S>,
    column_baseline_x0: f32,
    font_size: f32,
) -> bool
where
    S: MonospaceSpan,
{
    // Criterion 1: All spans must use monospace font
    for line in &block.lines {
        for span in &line.spans {
            if !span.is_monospace() {
                return false;
            }
        }
    }

    // Criterion 2: Block must be indented ≥ 2em from column baseline
    let em_width = font_size;
    let indent_threshold = 2.0 * em_width;
    let block_x0 = block.bbox[0];

    block_x0 >= column_baseline_x0 + indent_threshold
}

/// Trait for spans that can report monospace status.
///
/// This trait allows the code classification logic to work with different
/// span representations while abstracting over font information access.
pub trait MonospaceSpan {
    /// Check if this span uses a monospace font.
    fn is_monospace(&self) -> bool;
}

/// Compute the column baseline x0 from a set of blocks.
///
/// The column baseline is the median x0 of all non-code paragraph blocks
/// in the column. This represents the typical left edge of body text.
///
/// # Arguments
///
/// * `blocks` - Blocks in the column
///
/// # Returns
///
/// The median x0 coordinate of non-code paragraph blocks, or 0.0 if no such blocks exist.
///
/// # Examples
///
/// ```ignore
/// use pdftract_core::layout::code::compute_column_baseline;
///
/// let blocks = vec![
///     make_paragraph_block(72.0), // x0 = 72
///     make_paragraph_block(72.0), // x0 = 72
///     make_paragraph_block(100.0), // x0 = 100 (indented)
/// ];
///
/// let baseline = compute_column_baseline(&blocks);
/// assert_eq!(baseline, 72.0); // Median of [72, 72, 100]
/// ```
pub fn compute_column_baseline<S>(blocks: &[crate::layout::line::Block<S>]) -> f32
where
    S: MonospaceSpan,
{
    // Collect x0 values from non-code paragraph blocks
    let mut x0_values: Vec<f32> = blocks
        .iter()
        .filter(|b| b.kind == "paragraph")
        .map(|b| b.bbox[0])
        .collect();

    if x0_values.is_empty() {
        return 0.0;
    }

    // Compute median
    x0_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    x0_values[x0_values.len() / 2]
}

/// Classify all blocks on a page, updating their kinds to "code" where appropriate.
///
/// This function processes blocks in column order and classifies each block
/// based on monospace font usage and indentation.
///
/// # Arguments
///
/// * `blocks` - Mutable slice of blocks to classify
///
/// # Algorithm
///
/// 1. Compute column baseline x0 from non-code paragraph blocks
/// 2. For each block, check if it meets code criteria
/// 3. Update block.kind to "code" if criteria are met
pub fn classify_page_code_blocks<S>(blocks: &mut [crate::layout::line::Block<S>])
where
    S: MonospaceSpan,
{
    // Compute column baseline x0 (median of non-code paragraph blocks)
    let column_baseline_x0 = compute_column_baseline(blocks);

    // Classify each block
    for block in blocks.iter_mut() {
        if block.kind == "paragraph" {
            let font_size = block.median_font_size;
            if classify_code(block, column_baseline_x0, font_size) {
                block.kind = "code".to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::line::{Block, Line};

    /// Test helper: create a mock span with monospace info.
    #[derive(Debug, Clone)]
    struct TestSpan {
        font_name: String,
        flags: Option<u32>,
    }

    impl MonospaceSpan for TestSpan {
        fn is_monospace(&self) -> bool {
            is_monospace_span(&self.font_name, self.flags)
        }
    }

    /// Test helper: create a mock line.
    fn make_test_line(spans: Vec<TestSpan>) -> Line<TestSpan> {
        Line {
            spans,
            bbox: [0.0, 0.0, 100.0, 12.0],
            baseline: 2.4,
            direction: crate::layout::line::LineDirection::Ltr,
            page_relative_y: 0.5,
            median_font_size: 12.0,
            rendering_mode: None,
            column: Some(0),
        }
    }

    /// Test helper: create a mock block.
    fn make_test_block(spans: Vec<TestSpan>, x0: f32, kind: &str) -> Block<TestSpan> {
        Block {
            lines: vec![make_test_line(spans)],
            kind: kind.to_string(),
            text: String::new(),
            bbox: [x0, 0.0, x0 + 100.0, 12.0],
            median_font_size: 12.0,
            column: 0,
        }
    }

    #[test]
    fn test_is_monospace_font_name_courier() {
        assert!(is_monospace_font_name("Courier"));
        assert!(is_monospace_font_name("Courier-New"));
        assert!(is_monospace_font_name("ABCDEF+Courier")); // Subset prefix
    }

    #[test]
    fn test_is_monospace_font_name_mono() {
        assert!(is_monospace_font_name("Mono"));
        assert!(is_monospace_font_name("SourceCodePro"));
        assert!(is_monospace_font_name("LiberationMono"));
    }

    #[test]
    fn test_is_monospace_font_name_code() {
        assert!(is_monospace_font_name("Code"));
        assert!(is_monospace_font_name("SourceCodePro"));
        assert!(is_monospace_font_name("FiraCode"));
    }

    #[test]
    fn test_is_monospace_font_name_fixed() {
        assert!(is_monospace_font_name("Fixed"));
        assert!(is_monospace_font_name("Fixedsys"));
    }

    #[test]
    fn test_is_monospace_font_name_console() {
        assert!(is_monospace_font_name("Console"));
    }

    #[test]
    fn test_is_not_monospace_font_name() {
        assert!(!is_monospace_font_name("Times-Roman"));
        assert!(!is_monospace_font_name("Helvetica"));
        assert!(!is_monospace_font_name("Arial"));
    }

    #[test]
    fn test_is_fixed_pitch_flag() {
        assert!(is_fixed_pitch_flag(Some(1))); // Bit 0 set
        assert!(is_fixed_pitch_flag(Some(0b00000001)));
        assert!(is_fixed_pitch_flag(Some(0b11111111))); // All bits set
        assert!(!is_fixed_pitch_flag(Some(0))); // Bit 0 not set
        assert!(!is_fixed_pitch_flag(Some(2))); // Bit 1 set, not bit 0
        assert!(!is_fixed_pitch_flag(None)); // No flags
    }

    #[test]
    fn test_is_monospace_span_name_only() {
        // Font name indicates monospace, no flags
        assert!(is_monospace_span("Courier", None));
        assert!(is_monospace_span("Mono", None));
    }

    #[test]
    fn test_is_monospace_span_flags_only() {
        // FixedPitch flag set, non-monospace name
        assert!(is_monospace_span("CustomFont", Some(1)));
    }

    #[test]
    fn test_is_not_monospace_span() {
        // Neither name nor flags indicate monospace
        assert!(!is_monospace_span("Times-Roman", None));
        assert!(!is_monospace_span("Times-Roman", Some(0)));
    }

    #[test]
    fn test_classify_code_all_courier_indented() {
        // All-Courier block indented 24pt with font_size 12pt (2em=24pt)
        let spans = vec![TestSpan {
            font_name: "Courier".to_string(),
            flags: None,
        }];
        let block = make_test_block(spans, 96.0, "paragraph"); // x0 = 96

        // Column baseline at 72, block at 96: indent = 24pt = 2em
        assert!(classify_code(&block, 72.0, 12.0));
    }

    #[test]
    fn test_classify_code_not_indented() {
        // All-monospace block but not indented enough
        let spans = vec![TestSpan {
            font_name: "Courier".to_string(),
            flags: None,
        }];
        let block = make_test_block(spans, 80.0, "paragraph"); // x0 = 80

        // Column baseline at 72, block at 80: indent = 8pt < 2em (24pt)
        assert!(!classify_code(&block, 72.0, 12.0));
    }

    #[test]
    fn test_classify_code_mixed_font() {
        // Mixed serif+monospace -> NOT code
        let spans = vec![
            TestSpan {
                font_name: "Courier".to_string(),
                flags: None,
            },
            TestSpan {
                font_name: "Times-Roman".to_string(),
                flags: None,
            },
        ];
        let block = make_test_block(spans, 96.0, "paragraph");

        assert!(!classify_code(&block, 72.0, 12.0));
    }

    #[test]
    fn test_classify_code_one_serif_at_end() {
        // One serif span at end -> NOT code
        let spans = vec![
            TestSpan {
                font_name: "Courier".to_string(),
                flags: None,
            },
            TestSpan {
                font_name: "Courier".to_string(),
                flags: None,
            },
            TestSpan {
                font_name: "Times-Roman".to_string(),
                flags: None,
            },
        ];
        let block = make_test_block(spans, 96.0, "paragraph");

        assert!(!classify_code(&block, 72.0, 12.0));
    }

    #[test]
    fn test_classify_code_fixed_pitch_flag() {
        // FixedPitch flag set, no "Mono" in name -> STILL code
        let spans = vec![TestSpan {
            font_name: "CustomFont".to_string(),
            flags: Some(1),
        }];
        let block = make_test_block(spans, 96.0, "paragraph");

        assert!(classify_code(&block, 72.0, 12.0));
    }

    #[test]
    fn test_compute_column_baseline() {
        let blocks = vec![
            make_test_block(
                vec![TestSpan {
                    font_name: "Times-Roman".to_string(),
                    flags: None,
                }],
                72.0,
                "paragraph",
            ),
            make_test_block(
                vec![TestSpan {
                    font_name: "Times-Roman".to_string(),
                    flags: None,
                }],
                72.0,
                "paragraph",
            ),
            make_test_block(
                vec![TestSpan {
                    font_name: "Times-Roman".to_string(),
                    flags: None,
                }],
                100.0,
                "paragraph",
            ),
        ];

        let baseline = compute_column_baseline(&blocks);
        assert_eq!(baseline, 72.0); // Median of [72, 72, 100]
    }

    #[test]
    fn test_compute_column_baseline_empty() {
        let blocks: Vec<Block<TestSpan>> = vec![];

        let baseline = compute_column_baseline(&blocks);
        assert_eq!(baseline, 0.0);
    }

    #[test]
    fn test_compute_column_baseline_no_paragraphs() {
        let blocks = vec![
            make_test_block(
                vec![TestSpan {
                    font_name: "Courier".to_string(),
                    flags: None,
                }],
                72.0,
                "heading",
            ),
            make_test_block(
                vec![TestSpan {
                    font_name: "Courier".to_string(),
                    flags: None,
                }],
                72.0,
                "list",
            ),
        ];

        let baseline = compute_column_baseline(&blocks);
        assert_eq!(baseline, 0.0); // No paragraph blocks
    }

    #[test]
    fn test_classify_page_code_blocks() {
        let mut blocks = vec![
            // Regular paragraph at baseline
            make_test_block(
                vec![TestSpan {
                    font_name: "Times-Roman".to_string(),
                    flags: None,
                }],
                72.0,
                "paragraph",
            ),
            // Indented monospace block -> should become code
            make_test_block(
                vec![TestSpan {
                    font_name: "Courier".to_string(),
                    flags: None,
                }],
                96.0,
                "paragraph",
            ),
            // Non-indented monospace block -> should stay paragraph
            make_test_block(
                vec![TestSpan {
                    font_name: "Courier".to_string(),
                    flags: None,
                }],
                72.0,
                "paragraph",
            ),
        ];

        classify_page_code_blocks(&mut blocks);

        assert_eq!(blocks[0].kind, "paragraph"); // Unchanged
        assert_eq!(blocks[1].kind, "code"); // Upgraded to code
        assert_eq!(blocks[2].kind, "paragraph"); // Not indented enough
    }
}
