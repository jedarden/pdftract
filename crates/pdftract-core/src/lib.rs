//! pdftract-core — Core PDF parsing and text extraction primitives.
//!
//! This crate provides the foundational data structures and parsers for
//! processing PDF documents, including the lexer, object parser, and
//! text extraction engines.

pub mod attachment;
pub mod cache;
pub mod classify;
pub mod diagnostics;
pub mod document;
#[cfg(feature = "ocr")]
pub mod dpi;
pub mod extract;
pub mod fingerprint;
pub mod font;
pub mod graphics_state;
#[cfg(feature = "ocr")]
pub mod hybrid;
pub mod layout;
pub mod markdown;
#[cfg(feature = "ocr")]
pub mod ocr;
pub mod options;
pub mod parser;
#[cfg(feature = "ocr")]
pub mod preprocess;
#[cfg(feature = "profiles")]
pub mod profiles;
pub mod receipts;
#[cfg(feature = "ocr")]
pub mod render;
#[cfg(feature = "remote")]
pub mod url_validation;

// Re-export has_full_render for runtime feature detection
#[cfg(all(feature = "ocr", feature = "full-render"))]
pub use render::pdfium_path::has_full_render;
pub mod schema;
pub mod semaphore;
pub mod signature;
pub mod table;

// Re-export key types for convenience
pub use document::{PageExtraction, PageIter, PdfExtractor};
pub use extract::{
    extract_pdf, extract_pdf_ndjson, ExtractionMetadata, ExtractionResult, PageResult,
};
pub use font::std14::{get_std14_metrics, NamedEncoding, Std14Metrics};
pub use markdown::{block_to_markdown, page_to_markdown, parse_anchors, Anchor};
pub use options::{ExtractionOptions, ReceiptsMode};
pub use parser::pages::{count_pages_tree, LazyPageIter, PageDict, DEFAULT_MEDIABOX};
pub use schema::{BlockJson, CellJson, ExtractionQuality, RowJson, SpanJson, SpanRef, TableJson};
pub use table::{GridCandidate, PageContext as TablePageContext, TableDetector};

#[cfg(feature = "ocr")]
pub use dpi::{select_dpi, FontSizeSpan, Pdf1Filter};
#[cfg(feature = "ocr")]
pub use hybrid::{
    compute_cell_crops, compute_iou, crop_cell_from_page, get_hybrid_cells,
    merge_vector_and_ocr_spans, CellCrop, Span, SpanSource,
};
#[cfg(feature = "ocr")]
pub use ocr::{
    borrow_or_init, calculate_wer, detect_available_languages, init_count, parse_hocr,
    reset_init_count, run_tesseract, run_tesseract_on_cell, validate_ocr_languages, HocrWord,
    TessOpts,
};
#[cfg(feature = "ocr")]
pub use preprocess::{
    add_border_padding, binarize_otsu, binarize_sauvola, denoise_median, deskew,
    normalize_contrast, preprocess, ImageSource,
};
