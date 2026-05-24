//! pdftract-core — Core PDF parsing and text extraction primitives.
//!
//! This crate provides the foundational data structures and parsers for
//! processing PDF documents, including the lexer, object parser, and
//! text extraction engines.

pub mod cache;
pub mod classify;
pub mod diagnostics;
#[cfg(feature = "ocr")]
pub mod dpi;
pub mod document;
#[cfg(feature = "ocr")]
pub mod preprocess;
pub mod extract;
pub mod fingerprint;
pub mod font;
pub mod graphics_state;
#[cfg(feature = "ocr")]
pub mod hybrid;
pub mod options;
pub mod parser;
pub mod receipts;
#[cfg(feature = "ocr")]
pub mod render;

// Re-export has_full_render for runtime feature detection
#[cfg(all(feature = "ocr", feature = "full-render"))]
pub use render::pdfium_path::has_full_render;
pub mod schema;
pub mod semaphore;
pub mod table;

// Re-export key types for convenience
pub use document::{PdfExtractor, PageIter, PageExtraction};
pub use extract::{extract_pdf, extract_pdf_ndjson, ExtractionResult, PageResult, ExtractionMetadata};
pub use font::std14::{Std14Metrics, NamedEncoding, get_std14_metrics};
pub use options::{ExtractionOptions, ReceiptsMode};
pub use parser::pages::{LazyPageIter, PageDict, DEFAULT_MEDIABOX, count_pages_tree};
pub use schema::{SpanJson, BlockJson, ExtractionQuality};
pub use table::{TableDetector, PageContext as TablePageContext, GridCandidate};

#[cfg(feature = "ocr")]
pub use dpi::{Pdf1Filter, FontSizeSpan, select_dpi};
#[cfg(feature = "ocr")]
pub use hybrid::{Span, SpanSource, compute_iou, merge_vector_and_ocr_spans, crop_cell_from_page, get_hybrid_cells, compute_cell_crops, CellCrop};
