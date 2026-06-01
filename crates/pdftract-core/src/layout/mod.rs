//! Layout analysis for Phase 4.
//!
//! This module implements block-level layout analysis including:
//! - Caption classification (caption.rs)
//! - Code block classification (code.rs)
//! - Column label assignment (columns.rs)
//! - Figure classification (figure.rs)
//! - Line formation (line.rs)
//! - Reading order determination via XY-cut (reading_order.rs)
//! - Readability aggregation (readability.rs)
//! - English wordlist for dict coverage scoring (wordlist.rs)
//! - Text correction pipeline (correction.rs)
//! - Watermark and Formula stub classifiers (watermark_formula.rs)
//!
//! Phase 4 organizes extracted text into semantic blocks (paragraphs,
//! headings, figures, captions, etc.) based on spatial and font metrics.

pub mod caption;
pub mod code;
pub mod columns;
pub mod correction;
pub mod figure;
pub mod header_footer;
pub mod line;
pub mod readability;
pub mod reading_order;
pub mod watermark_formula;
pub mod wordlist;

pub use caption::{classify_caption, classify_page_captions, Block, PageContext};
pub use code::{
    classify_code, classify_page_code_blocks, is_fixed_pitch_flag, is_monospace_font_name,
    is_monospace_span, MonospaceSpan,
};
pub use columns::{assign_columns_to_lines, assign_columns_to_spans, build_x0_histogram, Column, ColumnGap};
pub use correction::{detect_and_repair_mojibake, repair_hyphenation, HyphenableSpan};
pub use figure::{classify_figure, FigurePageContext};
pub use header_footer::detect_headers_and_footers;
pub use line::{
    cluster_spans_into_lines, compute_baseline, group_lines_into_blocks, union_bboxes, BlockInput,
    HasBBox, HasFontSize, Line, LineDirection, LineMetadata,
};
pub use readability::{aggregate_page_readability, ScoredSpan};
pub use reading_order::{xy_cut, BlockWithBBox, HasBBox as HasBBoxForOrder, XYCutResult};
pub use watermark_formula::{classify_formula, classify_watermark};
pub use wordlist::is_english_word;
