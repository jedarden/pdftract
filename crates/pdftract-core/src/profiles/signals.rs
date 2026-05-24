//! Feature signal extraction for document type classification (Phase 5.6.3).
//!
//! This module implements the signal extractor that computes all features
//! the classifier needs in a single pass during Phase 4 assembly.
//!
//! ## Signals Computed
//!
//! - **Text pattern hits**: Currency symbols, ISO dates, keywords (INVOICE, WHEREAS, Abstract, References, etc.)
//! - **Page count**: Total number of pages
//! - **Table density**: Fraction of blocks with `kind: "table"`
//! - **Heading hierarchy depth**: Maximum heading nesting level (H1, H2, etc.)
//! - **Font diversity**: Count of distinct font names used in the document
//! - **Glyph density**: Mean ratio of extracted characters to expected characters per page
//! - **Presence flags**: Signature field, form field, math operators, bullet lists, footer page numbers
//!
//! ## Performance
//!
//! Signal extraction is designed to be < 1% of total extraction time for a
//! 100-page document. Text patterns are compiled once via `OnceLock` and
//! reused across all pages.

use crate::profiles::engine::FeatureSignals;
use crate::schema::{BlockJson, SpanJson};
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

// Static regex patterns compiled once and reused
// These are marked as allow(dead_code) because they're accessed through the
// public getter functions below and used in tests
#[allow(dead_code)]
static CURRENCY_REGEX: OnceLock<Regex> = OnceLock::new();

/// ISO date pattern regex: YYYY-MM-DD format.
#[allow(dead_code)]
static ISO_DATE_REGEX: OnceLock<Regex> = OnceLock::new();

/// Invoice keyword pattern regex (case-insensitive).
#[allow(dead_code)]
static INVOICE_REGEX: OnceLock<Regex> = OnceLock::new();

/// "Whereas" keyword pattern regex (case-insensitive, for contracts).
#[allow(dead_code)]
static WHEREAS_REGEX: OnceLock<Regex> = OnceLock::new();

/// "Abstract" heading pattern regex (case-insensitive, for scientific papers).
#[allow(dead_code)]
static ABSTRACT_REGEX: OnceLock<Regex> = OnceLock::new();

/// "References" heading pattern regex (case-insensitive, for scientific papers).
#[allow(dead_code)]
static REFERENCES_REGEX: OnceLock<Regex> = OnceLock::new();

/// Page number pattern regex: standalone numbers or "Page N" patterns.
#[allow(dead_code)]
static PAGE_NUMBER_REGEX: OnceLock<Regex> = OnceLock::new();

/// Bullet list pattern regex: bullet characters (•, -, *, etc.).
#[allow(dead_code)]
static BULLET_REGEX: OnceLock<Regex> = OnceLock::new();

/// Math operator pattern regex: ∫, ∑, ∏, √, ±, ×, ÷, etc.
#[allow(dead_code)]
static MATH_OPERATOR_REGEX: OnceLock<Regex> = OnceLock::new();

/// Initialize the currency regex.
fn currency_regex() -> &'static Regex {
    CURRENCY_REGEX
        .get_or_init(|| Regex::new(r"[\$€£¥]\s*\d").unwrap_or_else(|_| Regex::new(r"$").unwrap()))
}

/// Initialize the ISO date regex.
fn iso_date_regex() -> &'static Regex {
    ISO_DATE_REGEX.get_or_init(|| {
        Regex::new(r"\b\d{4}-\d{2}-\d{2}\b").unwrap_or_else(|_| Regex::new(r"\b").unwrap())
    })
}

/// Initialize the invoice keyword regex.
fn invoice_regex() -> &'static Regex {
    INVOICE_REGEX.get_or_init(|| {
        Regex::new(r"(?i)invoice\s*#?").unwrap_or_else(|_| Regex::new(r"\b").unwrap())
    })
}

/// Initialize the whereas keyword regex.
fn whereas_regex() -> &'static Regex {
    WHEREAS_REGEX.get_or_init(|| {
        Regex::new(r"(?i)whereas[,\s]").unwrap_or_else(|_| Regex::new(r"\b").unwrap())
    })
}

/// Initialize the abstract heading regex.
fn abstract_regex() -> &'static Regex {
    ABSTRACT_REGEX.get_or_init(|| {
        Regex::new(r"(?i)^\s*abstract\s*$").unwrap_or_else(|_| Regex::new(r"\b").unwrap())
    })
}

/// Initialize the references heading regex.
fn references_regex() -> &'static Regex {
    REFERENCES_REGEX.get_or_init(|| {
        Regex::new(r"(?i)^\s*references\s*$").unwrap_or_else(|_| Regex::new(r"\b").unwrap())
    })
}

/// Initialize the page number regex.
fn page_number_regex() -> &'static Regex {
    PAGE_NUMBER_REGEX.get_or_init(|| {
        // Match standalone numbers or "Page N" at the end of text
        // This avoids matching "Page 1 of 10" since that's followed by more text
        Regex::new(r"(?i)^\s*\d+\s*$|^Page\s+\d+\s*$")
            .unwrap_or_else(|_| Regex::new(r"\b").unwrap())
    })
}

/// Initialize the bullet list regex.
fn bullet_regex() -> &'static Regex {
    BULLET_REGEX.get_or_init(|| {
        Regex::new(r"^[\s\t]*[•\-\*●○►]\s+").unwrap_or_else(|_| Regex::new(r"\b").unwrap())
    })
}

/// Initialize the math operator regex.
fn math_operator_regex() -> &'static Regex {
    MATH_OPERATOR_REGEX.get_or_init(|| {
        Regex::new(r"[∫∫∫∑∏√±×÷≈≠≤≥∂∇∞∪∩]").unwrap_or_else(|_| Regex::new(r"\b").unwrap())
    })
}

/// Per-page signal accumulator.
///
/// Collects signal contributions from a single page during extraction.
/// These are aggregated into document-level `FeatureSignals`.
#[derive(Debug, Clone, Default)]
pub struct PageSignalAccumulator {
    /// Text content for this page.
    pub text: String,
    /// Font names used on this page.
    pub fonts: HashSet<String>,
    /// Number of blocks classified as tables.
    pub table_count: u32,
    /// Maximum heading depth on this page (1 = H1, 2 = H2, etc.).
    pub heading_depth: u8,
    /// Glyph density ratio for this page.
    pub glyph_density: Option<f32>,
    /// Whether this page has bullet lists.
    pub has_bullets: bool,
    /// Whether this page has footer page numbers.
    pub has_footer_page_numbers: bool,
    /// Whether this page has math operators.
    pub has_math_operators: bool,
}

impl PageSignalAccumulator {
    /// Create a new empty page signal accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Extract signals from a page's blocks and spans.
    ///
    /// This is the main entry point for per-page signal extraction.
    /// It should be called during Phase 4 assembly after blocks are formed.
    ///
    /// # Arguments
    ///
    /// * `blocks` - Blocks extracted from this page
    /// * `spans` - Spans extracted from this page
    ///
    /// # Returns
    ///
    /// A `PageSignalAccumulator` with signal data for this page.
    pub fn extract_from_page(blocks: &[BlockJson], spans: &[SpanJson]) -> Self {
        let mut accumulator = Self::new();

        // Collect text content from all spans
        for span in spans {
            accumulator.text.push_str(&span.text);
            accumulator.text.push(' ');
            accumulator.fonts.insert(span.font.clone());
        }

        // Analyze blocks for structural signals
        for block in blocks {
            // Count table blocks
            if block.kind == "table" {
                accumulator.table_count += 1;
            }

            // Track heading depth
            if let Some(level) = block.level {
                accumulator.heading_depth = accumulator.heading_depth.max(level);
            }

            // Check for bullet lists (heuristic: block text starts with bullet)
            if bullet_regex().is_match(&block.text) {
                accumulator.has_bullets = true;
            }

            // Check for footer page numbers (last blocks on page)
            // This is a heuristic: short text with just numbers or "Page N"
            if block.text.len() < 50 && page_number_regex().is_match(&block.text) {
                accumulator.has_footer_page_numbers = true;
            }
        }

        // Check for math operators in the text
        if math_operator_regex().is_match(&accumulator.text) {
            accumulator.has_math_operators = true;
        }

        // Compute glyph density (placeholder - requires expected character count)
        // For now, use a simple heuristic based on text length vs font size
        if !spans.is_empty() {
            let total_chars: usize = spans.iter().map(|s| s.text.chars().count()).sum();
            let bbox_area: f64 = spans
                .iter()
                .map(|s| {
                    let width = s.bbox[2] - s.bbox[0];
                    let height = s.bbox[3] - s.bbox[1];
                    width * height
                })
                .sum();
            // Very rough heuristic: chars per square point
            accumulator.glyph_density = if bbox_area > 0.0 {
                Some((total_chars as f32) / (bbox_area as f32))
            } else {
                None
            };
        }

        accumulator
    }
}

/// Extract document-level feature signals from all pages.
///
/// Aggregates per-page signal accumulators into a single `FeatureSignals`
/// struct that the classifier engine uses.
///
/// # Arguments
///
/// * `pages` - Slice of (blocks, spans) tuples for each page
/// * `has_signature_field` - Whether the document has any AcroForm signature fields
/// * `has_form_field` - Whether the document has any AcroForm fields (text, checkbox, etc.)
///
/// # Returns
///
/// A `FeatureSignals` struct populated with all computed signals.
pub fn extract_feature_signals(
    pages: &[(Vec<BlockJson>, Vec<SpanJson>)],
    has_signature_field: bool,
    has_form_field: bool,
) -> FeatureSignals {
    let mut signals = FeatureSignals::new();

    // Track font names across all pages
    let mut all_fonts: HashSet<String> = HashSet::new();

    // Track maximum heading depth
    let mut max_heading_depth: u8 = 0;

    // Track total table count
    let mut total_table_count: u32 = 0;

    // Track glyph density per page
    let mut glyph_densities: Vec<f32> = Vec::new();

    // Track presence flags
    let mut has_math_operators = false;
    let mut has_bullet_lists = false;
    let mut has_footer_page_numbers = false;

    // Collect text from all pages
    let mut full_text = String::new();

    // Process each page
    for (blocks, spans) in pages {
        // Extract signals from this page
        let page_acc = PageSignalAccumulator::extract_from_page(blocks, spans);

        // Aggregate document-level signals
        full_text.push_str(&page_acc.text);
        full_text.push('\n');

        all_fonts.extend(page_acc.fonts);
        max_heading_depth = max_heading_depth.max(page_acc.heading_depth);
        total_table_count += page_acc.table_count;

        if let Some(density) = page_acc.glyph_density {
            glyph_densities.push(density);
        }

        has_math_operators = has_math_operators || page_acc.has_math_operators;
        has_bullet_lists = has_bullet_lists || page_acc.has_bullets;
        has_footer_page_numbers = has_footer_page_numbers || page_acc.has_footer_page_numbers;
    }

    // Populate FeatureSignals
    signals.text = full_text;
    signals.page_count = pages.len() as u32;
    signals.table_block_count = total_table_count;
    signals.has_signature_field = has_signature_field;
    signals.has_form_field = has_form_field;
    signals.has_math_operators = has_math_operators;
    signals.has_bullet_lists = has_bullet_lists;
    signals.font_diversity = all_fonts.len() as u32;
    signals.heading_depth = max_heading_depth as u32;

    // Compute mean glyph density across pages
    signals.glyph_density = if glyph_densities.is_empty() {
        0.0
    } else {
        glyph_densities.iter().sum::<f32>() / glyph_densities.len() as f32
    };

    signals.has_footer_page_numbers = has_footer_page_numbers;

    // Build text pattern hits for fast matching
    signals.build_pattern_hits();

    signals
}

/// Extract feature signals from extraction results.
///
/// Convenience function that converts from the extraction pipeline's
/// `PageResult` format to the signals format.
///
/// # Arguments
///
/// * `page_results` - Slice of page results with blocks and spans
/// * `has_signature_field` - Whether the document has any AcroForm signature fields
/// * `has_form_field` - Whether the document has any AcroForm fields
///
/// # Returns
///
/// A `FeatureSignals` struct populated with all computed signals.
pub fn extract_signals_from_results(
    page_results: &[(Vec<BlockJson>, Vec<SpanJson>)],
    has_signature_field: bool,
    has_form_field: bool,
) -> FeatureSignals {
    extract_feature_signals(page_results, has_signature_field, has_form_field)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_span(text: &str, font: &str) -> SpanJson {
        SpanJson {
            text: text.to_string(),
            bbox: [0.0, 0.0, 100.0, 12.0],
            font: font.to_string(),
            size: 12.0,
            confidence: None,
            receipt: None,
        }
    }

    fn make_test_block(kind: &str, text: &str, level: Option<u8>) -> BlockJson {
        BlockJson {
            kind: kind.to_string(),
            text: text.to_string(),
            bbox: [0.0, 0.0, 100.0, 50.0],
            level,
            table_index: None,
            receipt: None,
        }
    }

    #[test]
    fn test_currency_regex_matches() {
        let regex = currency_regex();
        assert!(regex.is_match("$100"));
        assert!(regex.is_match("€ 99"));
        assert!(regex.is_match("£50.00"));
        assert!(regex.is_match("¥1000"));
        assert!(!regex.is_match("100"));
    }

    #[test]
    fn test_iso_date_regex_matches() {
        let regex = iso_date_regex();
        assert!(regex.is_match("2024-01-15"));
        assert!(regex.is_match("Date: 2023-12-31"));
        assert!(!regex.is_match("01/15/2024"));
        assert!(!regex.is_match("15-01-2024"));
    }

    #[test]
    fn test_invoice_regex_matches() {
        let regex = invoice_regex();
        assert!(regex.is_match("INVOICE #123"));
        assert!(regex.is_match("Invoice INV-001"));
        assert!(regex.is_match("invoice total"));
        assert!(!regex.is_match("RECEIPT #123"));
    }

    #[test]
    fn test_whereas_regex_matches() {
        let regex = whereas_regex();
        assert!(regex.is_match("WHEREAS, the parties agree"));
        assert!(regex.is_match("Whereas the Seller"));
        assert!(regex.is_match("whereas, the Buyer"));
        assert!(!regex.is_match("The parties agree"));
    }

    #[test]
    fn test_abstract_regex_matches() {
        let regex = abstract_regex();
        assert!(regex.is_match("Abstract"));
        assert!(regex.is_match("  Abstract  "));
        assert!(regex.is_match("ABSTRACT"));
        assert!(!regex.is_match("Abstract: This is..."));
    }

    #[test]
    fn test_references_regex_matches() {
        let regex = references_regex();
        assert!(regex.is_match("References"));
        assert!(regex.is_match("  References  "));
        assert!(regex.is_match("REFERENCES"));
        assert!(!regex.is_match("References: [1] Smith"));
    }

    #[test]
    fn test_page_number_regex_matches() {
        let regex = page_number_regex();
        assert!(regex.is_match("1"));
        assert!(regex.is_match("  42  "));
        assert!(regex.is_match("Page 1"));
        assert!(regex.is_match("PAGE 10"));
        // "Page 1 of 10" doesn't match because the pattern requires the text to end after the number
        assert!(!regex.is_match("Page 1 of 10"));
        assert!(!regex.is_match("123 Main St"));
    }

    #[test]
    fn test_bullet_regex_matches() {
        let regex = bullet_regex();
        assert!(regex.is_match("• Item 1"));
        assert!(regex.is_match("- Item 2"));
        assert!(regex.is_match("* Item 3"));
        assert!(regex.is_match("  ● Item 4"));
        assert!(!regex.is_match("Item 1"));
    }

    #[test]
    fn test_math_operator_regex_matches() {
        let regex = math_operator_regex();
        assert!(regex.is_match("∫ x dx"));
        assert!(regex.is_match("∑_{i=0}^n"));
        assert!(regex.is_match("x ≠ y"));
        assert!(regex.is_match("x ± y"));
        assert!(!regex.is_match("x + y"));
    }

    #[test]
    fn test_page_signal_accumulator_extract_from_page() {
        let blocks = vec![
            make_test_block("paragraph", "This is a paragraph.", None),
            make_test_block("heading", "Introduction", Some(1)),
            make_test_block("table", "Table data", None),
        ];

        let spans = vec![
            make_test_span("This is a paragraph.", "Helvetica"),
            make_test_span("Introduction", "Helvetica-Bold"),
        ];

        let acc = PageSignalAccumulator::extract_from_page(&blocks, &spans);

        assert_eq!(acc.table_count, 1);
        assert_eq!(acc.heading_depth, 1);
        assert!(acc.fonts.contains("Helvetica"));
        assert!(acc.fonts.contains("Helvetica-Bold"));
        assert!(acc.text.contains("paragraph"));
        assert!(acc.text.contains("Introduction"));
    }

    #[test]
    fn test_page_signal_accumulator_bullet_detection() {
        let blocks = vec![
            make_test_block("paragraph", "• Item 1", None),
            make_test_block("paragraph", "- Item 2", None),
            make_test_block("paragraph", "* Item 3", None),
        ];

        let spans = vec![
            make_test_span("• Item 1", "Helvetica"),
            make_test_span("- Item 2", "Helvetica"),
            make_test_span("* Item 3", "Helvetica"),
        ];

        let acc = PageSignalAccumulator::extract_from_page(&blocks, &spans);

        assert!(acc.has_bullets);
    }

    #[test]
    fn test_page_signal_accumulator_page_number_detection() {
        let blocks = vec![
            make_test_block("paragraph", "1", None),
            make_test_block("paragraph", "  42  ", None),
            make_test_block("paragraph", "Page 10", None),
        ];

        let spans = vec![
            make_test_span("1", "Helvetica"),
            make_test_span("42", "Helvetica"),
            make_test_span("Page 10", "Helvetica"),
        ];

        let acc = PageSignalAccumulator::extract_from_page(&blocks, &spans);

        assert!(acc.has_footer_page_numbers);
    }

    #[test]
    fn test_extract_feature_signals_basic() {
        let pages = vec![
            (
                vec![make_test_block("paragraph", "Page 1 content", None)],
                vec![make_test_span("Page 1 content", "Helvetica")],
            ),
            (
                vec![
                    make_test_block("paragraph", "Page 2 content", None),
                    make_test_block("table", "Table data", None),
                ],
                vec![make_test_span("Page 2 content", "Times-Roman")],
            ),
        ];

        let signals = extract_feature_signals(&pages, false, false);

        assert_eq!(signals.page_count, 2);
        assert_eq!(signals.table_block_count, 1);
        assert_eq!(signals.heading_depth, 0);
        assert!(signals.text.contains("Page 1 content"));
        assert!(signals.text.contains("Page 2 content"));
    }

    #[test]
    fn test_extract_feature_signals_with_heading_depth() {
        let pages = vec![(
            vec![
                make_test_block("heading", "H1", Some(1)),
                make_test_block("heading", "H2", Some(2)),
                make_test_block("heading", "H3", Some(3)),
            ],
            vec![
                make_test_span("H1", "Helvetica-Bold"),
                make_test_span("H2", "Helvetica-Bold"),
                make_test_span("H3", "Helvetica-Bold"),
            ],
        )];

        let signals = extract_feature_signals(&pages, false, false);

        assert_eq!(signals.heading_depth, 3);
    }

    #[test]
    fn test_extract_feature_signals_font_diversity() {
        let pages = vec![(
            vec![make_test_block("paragraph", "Text", None)],
            vec![
                make_test_span("Text", "Helvetica"),
                make_test_span("Text", "Times-Roman"),
                make_test_span("Text", "Courier"),
            ],
        )];

        let signals = extract_feature_signals(&pages, false, false);

        assert_eq!(signals.font_diversity, 3);
    }

    #[test]
    fn test_extract_feature_signals_presence_flags() {
        let pages = vec![(
            vec![
                make_test_block("paragraph", "∫ x dx", None),
                make_test_block("paragraph", "• Item", None),
                make_test_block("paragraph", "Page 1", None),
            ],
            vec![make_test_span("∫ x dx • Item Page 1", "Helvetica")],
        )];

        let signals = extract_feature_signals(&pages, true, true);

        assert!(signals.has_math_operators);
        assert!(signals.has_bullet_lists);
        assert!(signals.has_footer_page_numbers);
        assert!(signals.has_signature_field);
        assert!(signals.has_form_field);
    }

    #[test]
    fn test_extract_feature_signals_builds_pattern_hits() {
        let pages = vec![(
            vec![
                make_test_block("paragraph", "INVOICE #123 Date: 2024-01-15", None),
                make_test_block("paragraph", "Abstract", Some(1)),
            ],
            vec![make_test_span(
                "INVOICE #123 Date: 2024-01-15 Abstract",
                "Helvetica",
            )],
        )];

        let signals = extract_feature_signals(&pages, false, false);

        // Pattern hits should be built automatically
        assert!(signals.text.contains("INVOICE"));
        assert!(signals.text.contains("2024-01-15"));
        assert!(signals.text.contains("Abstract"));

        // build_pattern_hits() was called, so contains() should work
        assert!(signals.contains("invoice") > 0 || signals.contains("INVOICE") > 0);
    }

    #[test]
    fn test_extract_signals_from_results_alias() {
        let pages = vec![(
            vec![make_test_block("paragraph", "Test", None)],
            vec![make_test_span("Test", "Helvetica")],
        )];

        let signals1 = extract_feature_signals(&pages, false, false);
        let signals2 = extract_signals_from_results(&pages, false, false);

        // Both functions should return identical results
        assert_eq!(signals1.page_count, signals2.page_count);
        assert_eq!(signals1.table_block_count, signals2.table_block_count);
    }

    #[test]
    fn test_signal_extraction_determinism() {
        let pages = vec![
            (
                vec![make_test_block("paragraph", "Page 1", None)],
                vec![make_test_span("Page 1", "Helvetica")],
            ),
            (
                vec![make_test_block("paragraph", "Page 2", None)],
                vec![make_test_span("Page 2", "Times-Roman")],
            ),
        ];

        let signals1 = extract_feature_signals(&pages, false, false);
        let signals2 = extract_feature_signals(&pages, false, false);

        // Extracting twice should produce identical results
        assert_eq!(signals1.page_count, signals2.page_count);
        assert_eq!(signals1.font_diversity, signals2.font_diversity);
        assert_eq!(signals1.table_block_count, signals2.table_block_count);
    }

    #[test]
    fn test_empty_pages_handling() {
        let pages: Vec<(Vec<BlockJson>, Vec<SpanJson>)> = vec![];

        let signals = extract_feature_signals(&pages, false, false);

        assert_eq!(signals.page_count, 0);
        assert_eq!(signals.table_block_count, 0);
        assert_eq!(signals.font_diversity, 0);
        assert_eq!(signals.heading_depth, 0);
        assert!(!signals.has_signature_field);
        assert!(!signals.has_form_field);
        assert!(!signals.has_math_operators);
        assert!(!signals.has_bullet_lists);
        assert!(!signals.has_footer_page_numbers);
    }

    #[test]
    fn test_invoice_pattern_hits() {
        let pages = vec![(
            vec![
                make_test_block("heading", "INVOICE #12345", None),
                make_test_block("paragraph", "Total: $1,234.56", None),
            ],
            vec![
                make_test_span("INVOICE #12345", "Helvetica-Bold"),
                make_test_span("Total: $1,234.56", "Helvetica"),
            ],
        )];

        let signals = extract_feature_signals(&pages, false, false);

        // Should have currency pattern
        assert!(currency_regex().is_match(&signals.text));

        // Should have invoice keyword
        assert!(invoice_regex().is_match(&signals.text));
    }

    #[test]
    fn test_scientific_paper_patterns() {
        let pages = vec![(
            vec![
                make_test_block("heading", "Abstract", Some(1)),
                make_test_block("paragraph", "∫ f(x) dx", None),
                make_test_block("heading", "References", Some(1)),
            ],
            vec![
                make_test_span("Abstract", "Times-Bold"),
                make_test_span("∫ f(x) dx", "Times-Roman"),
                make_test_span("References", "Times-Bold"),
            ],
        )];

        let signals = extract_feature_signals(&pages, false, false);

        // The abstract and references regex patterns match standalone headings
        // Check that the text contains these headings
        assert!(signals.text.contains("Abstract"));
        assert!(signals.text.contains("References"));

        // Verify the regex patterns work on the isolated heading text
        assert!(abstract_regex().is_match("Abstract"));
        assert!(references_regex().is_match("References"));

        // Should have math operators
        assert!(signals.has_math_operators);
    }

    #[test]
    fn test_contract_pattern_hits() {
        let pages = vec![(
            vec![make_test_block(
                "paragraph",
                "WHEREAS, the parties agree to the following terms.",
                None,
            )],
            vec![make_test_span(
                "WHEREAS, the parties agree to the following terms.",
                "Times-Roman",
            )],
        )];

        let signals = extract_feature_signals(&pages, false, false);

        // Should have whereas keyword
        assert!(whereas_regex().is_match(&signals.text));
    }
}
