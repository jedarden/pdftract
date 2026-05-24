//! Layout analysis for Phase 4.
//!
//! This module implements block-level layout analysis including:
//! - Caption classification (caption.rs)
//! - Code block classification (code.rs)
//! - Column label assignment (columns.rs)
//! - Line formation (line.rs)
//! - Readability aggregation (readability.rs)
//! - English wordlist for dict coverage scoring (wordlist.rs)
//! - Text correction pipeline (correction.rs)
//!
//! Phase 4 organizes extracted text into semantic blocks (paragraphs,
//! headings, figures, captions, etc.) based on spatial and font metrics.

pub mod caption;
pub mod code;
pub mod columns;
pub mod correction;
pub mod line;
pub mod readability;
pub mod wordlist;

pub use caption::{classify_caption, classify_page_captions, Block, PageContext};
pub use code::{
    classify_code, classify_page_code_blocks, is_fixed_pitch_flag, is_monospace_font_name,
    is_monospace_span, MonospaceSpan,
};
pub use columns::{assign_columns_to_lines, assign_columns_to_spans, Column};
pub use correction::{detect_and_repair_mojibake, repair_hyphenation, HyphenableSpan};
pub use line::{
    cluster_spans_into_lines, compute_baseline, group_lines_into_blocks, union_bboxes, BlockInput,
    HasBBox, HasFontSize, Line, LineDirection, LineMetadata,
};
pub use readability::{aggregate_page_readability, ScoredSpan};
pub use wordlist::is_english_word;
