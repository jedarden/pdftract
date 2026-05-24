//! pdftract-core — Core PDF parsing and text extraction primitives.
//!
//! This crate provides the foundational data structures and parsers for
//! processing PDF documents, including the lexer, object parser, and
//! text extraction engines.

pub mod attachment;
pub mod cache;
pub mod classify;
pub mod diagnostics;
#[cfg(feature = "remote")]
pub mod url_validation;
#[cfg(feature = "ocr")]
pub mod dpi;
pub mod document;
#[cfg(feature = "ocr")]
pub mod ocr;
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
pub use schema::{SpanJson, BlockJson, ExtractionQuality, TableJson, RowJson, CellJson, SpanRef};
pub use table::{TableDetector, PageContext as TablePageContext, GridCandidate};

#[cfg(feature = "ocr")]
pub use dpi::{Pdf1Filter, FontSizeSpan, select_dpi};
#[cfg(feature = "ocr")]
pub use hybrid::{Span, SpanSource, compute_iou, merge_vector_and_ocr_spans, crop_cell_from_page, get_hybrid_cells, compute_cell_crops, CellCrop};
#[cfg(feature = "ocr")]
pub use ocr::{
    TessOpts, borrow_or_init, init_count, reset_init_count, validate_ocr_languages,
    detect_available_languages, HocrWord, parse_hocr, run_tesseract, run_tesseract_on_cell,
    calculate_wer,
};
#[cfg(feature = "ocr")]
pub use preprocess::{ImageSource, add_border_padding, normalize_contrast, binarize_otsu, binarize_sauvola, denoise_median, preprocess, deskew};
