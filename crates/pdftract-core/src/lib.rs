#![deny(missing_docs)]
//! pdftract-core — Core PDF parsing and text extraction primitives.
//!
//! This crate provides the foundational data structures and parsers for
//! processing PDF documents, including the PDF lexer, object model parser,
//! content stream interpreter, and text extraction engines.
//!
//! # Overview
//!
//! pdftract-core is a pure-Rust PDF processing library that extracts structured
//! text, tables, and metadata from PDF documents. It handles the full PDF specification
//! including encrypted documents, embedded fonts, and complex page layouts.
//!
//! The crate is organized into several layers:
//! - **Parser layer** (`parser`) — Lexes and parses PDF binary format into object model
//! - **Content stream layer** (`content_stream`, `graphics_state`) — Interprets drawing operations
//! - **Text extraction layer** (`extract`, `glyph`, `span`) — Reconstructs text from drawing commands
//! - **Analysis layer** (`layout`, `table`, `classify`) — Detects structure (tables, blocks, page type)
//! - **Output layer** (`schema`, `markdown`, `text`) — Serializes to JSON/Markdown/text
//!
//! # Quick Start
//!
//! ## Basic Text Extraction
//!
//! ```rust,no_run
//! use pdftract_core::{extract_pdf, ExtractionOptions, OutputOptions};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Extract text from a PDF file
//! let result = extract_pdf(
//!     "document.pdf",
//!     &ExtractionOptions::default(),
//!     &OutputOptions::default()
//! )?;
//!
//! // Access extracted text per page
//! for (page_num, page_result) in result.pages.iter().enumerate() {
//!     println!("Page {}: {} chars extracted", page_num + 1, page_result.text.len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## JSON Output with Schema
//!
//! ```rust,no_run
//! use pdftract_core::{extract_pdf_ndjson, ExtractionOptions, OutputOptions};
//! use std::fs::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Extract to NDJSON (one JSON object per page)
//! let output = File::create("output.ndjson")?;
//! extract_pdf_ndjson(
//!     "document.pdf",
//!     &ExtractionOptions::default(),
//!     &OutputOptions::default(),
//!     output
//! )?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Streaming Extraction for Large Files
//!
//! ```rust,no_run
//! use pdftract_core::{extract_pdf_streaming, ExtractionOptions, OutputOptions};
//! use std::fs::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Stream pages one at a time (memory-efficient for large PDFs)
//! let mut output = File::create("output.ndjson")?;
//! extract_pdf_streaming(
//!     "large_document.pdf",
//!     &ExtractionOptions::default(),
//!     &OutputOptions::default(),
//!     &mut output
//! )?;
//! # Ok(())
//! # }
//! ```
//!
//! ## With OCR for Scanned PDFs
//!
//! ```rust,no_run
//! use pdftract_core::{extract_pdf, ExtractionOptions, OutputOptions};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Enable OCR via "ocr" feature
//! # #[cfg(feature = "ocr")]
//! let result = extract_pdf(
//!     "scanned.pdf",
//!     &ExtractionOptions {
//!         ocr_languages: vec!["eng".to_string()],
//!         ..Default::default()
//!     },
//!     &OutputOptions::default()
//! )?;
//! # Ok(())
//! # }
//! ```
//!
//! # Feature Flags
//!
//! | Feature | Description | Default |
//! |---------|-------------|---------|
//! | `serde` | JSON serialization support | ✓ |
//! | `decrypt` | Decryption of encrypted PDFs | ✓ |
//! | `quick-xml` | Conformance detection via XML metadata | ✓ |
//! | `ocr` | Tesseract OCR for scanned documents | - |
//! | `full-render` | PDFium-based rendering (requires external library) | - |
//! | `remote` | HTTP range fetching for remote PDFs | - |
//! | `profiles` | Profiling/timing instrumentation | - |
//! | `receipts` | Cryptographic receipt generation | - |
//! | `cjk` | CJK text extraction via predefined CMap registry | - |
//! | `schemars` | JSON Schema generation | - |
//!
//! # JSON Schema
//!
//! The output JSON schema is documented at:
//! <https://github.com/jedarden/pdftract/blob/main/crates/pdftract-core/SCHEMA.md>
//!
//! # Architecture
//!
//! ## Extraction Pipeline
//!
//! 1. **Source Loading** — [`source::PdfSource`] trait handles file/memory/HTTP inputs
//! 2. **Parser** — [`parser`] module lexes PDF binary format into object model
//! 3. **Xref Resolution** — Cross-reference table resolves object offsets
//! 4. **Catalog/Page Tree** — Document structure traversal
//! 5. **Content Stream Parsing** — Drawing operations interpreted
//! 6. **Glyph Reconstruction** — Text extracted from drawing commands
//! 7. **Span Merging** — Glyphs merged into logical text spans
//! 8. **Layout Analysis** — Blocks, tables, reading order detected
//! 9. **Serialization** — JSON/Markdown/text output
//!
//! ## Memory Behavior
//!
//! The crate uses lazy loading and streaming to minimize memory:
//! - [`PageIter`] loads pages on-demand, not all at once
//! - [`extract_pdf_streaming`] writes output incrementally
//! - [`MmapSource`] memory-maps files for zero-copy access
//!
//! # Error Handling
//!
//! Most functions return `anyhow::Result<T>` which wraps various error types:
//! - File I/O errors from opening/reading PDFs
//! - Parsing errors from malformed PDF structures
//! - Decryption errors for encrypted PDFs (when `decrypt` feature is enabled)
//! - JSON serialization errors when emitting structured output
//!
//! # Thread Safety
//!
//! The extraction pipeline is designed for single-threaded use, but you can
//! process multiple independent PDFs in parallel using rayon or similar.


pub mod annotation;
pub mod atomic_file_writer;
pub mod attachment;
pub mod audit;
pub mod cache;
pub mod classify;
pub mod cmap;
pub mod confidence;
pub mod conformance;
pub mod content_stream;
pub mod decoder;
pub mod detection;
pub mod diagnostics;
pub mod document;
#[cfg(feature = "ocr")]
pub mod dpi;
#[cfg(feature = "decrypt")]
pub mod encryption;
pub mod extract;
pub mod fingerprint;
pub mod font;
pub mod forms;
pub mod glyph;
pub mod graphics_state;
#[cfg(feature = "ocr")]
pub mod hybrid;
pub mod javascript;
pub mod layout;
pub mod log_policy;
pub mod markdown;
#[cfg(feature = "ocr")]
pub mod ocr;
pub mod options;
pub mod output;
pub mod page_class;
pub mod pages;
pub mod parser;
#[cfg(feature = "ocr")]
pub mod preprocess;
#[cfg(feature = "profiles")]
pub mod profiles;
pub mod receipts;
#[cfg(feature = "ocr")]
pub mod render;
#[cfg(feature = "remote")]
pub mod remote;
pub mod source;
pub mod text;
#[cfg(feature = "remote")]
pub mod url_validation;
pub mod word_boundary;

// Re-export has_full_render for runtime feature detection
#[cfg(all(feature = "ocr", feature = "full-render"))]
pub use render::pdfium_path::has_full_render;
pub mod schema;
pub mod semaphore;
pub mod signature;
pub mod span;
pub mod span_flags;
pub mod table;
pub mod threads;

// Re-export key types for convenience
pub use confidence::{map_confidence_source, ConfidenceSource};
pub use document::{Document, PageExtraction, PageIter, PdfExtractor};
pub use extract::{
    extract_pdf, extract_pdf_ndjson, extract_pdf_streaming, extract_text, ExtractionMetadata,
    ExtractionResult, PageResult,
};
pub use font::std14::{get_std14_metrics, NamedEncoding, Std14Metrics};
pub use forms::{
    combine, walk_acroform_fields, AcroFieldType, AcroFormField, ChoiceValue, FormFieldValue,
};
pub use markdown::{
    block_to_markdown, form_fields_to_markdown, page_to_markdown, parse_anchors, span_to_markdown,
    Anchor,
};
pub use options::{ExtractionOptions, OutputOptions, ReceiptsMode};
pub use page_class::{page_type_string, PageClass, PageClassification};
pub use parser::pages::{count_pages_tree, LazyPageIter, PageDict, DEFAULT_MEDIABOX};
pub use schema::{
    AttachmentJson, BeadJson, BlockJson, CellJson, ExtractionQuality, RowJson, SpanJson, SpanRef,
    TableJson, ThreadJson,
};
pub use table::{GridCandidate, PageContext as TablePageContext, TableDetector};
pub use text::{serialize_page_text, TextOptions};
pub use word_boundary::{TextState, WordBoundaryDetector, WordBoundaryManager};

// Re-export PdfSource types (pdftract-1mmq9)
// Note: PdfSource trait is available via pdftract_core::source::PdfSource to avoid conflict with parser::stream::PdfSource
pub use source::{FileSource, MmapSource};

#[cfg(feature = "remote")]
pub use source::{HttpRangeSource, RemoteOpts};

// Re-export Phase 3 Glyph types (pdftract-4j0ub)
pub use glyph::{emit_glyph, new_raw_glyph_list, Glyph};

// Re-export Phase 4.1 Span types (pdftract-31ag5)
pub use span::{CssHexColor, Span, merge_glyphs_to_spans};

#[cfg(feature = "ocr")]
pub use dpi::{select_dpi, FontSizeSpan, Pdf1Filter};
#[cfg(feature = "ocr")]
pub use hybrid::{
    compute_cell_crops, compute_iou, crop_cell_from_page, get_hybrid_cells,
    merge_vector_and_ocr_spans, CellCrop, HybridSpan, SpanSource,
};
#[cfg(feature = "ocr")]
pub use ocr::preprocessing::{
    histogram_stretch, histogram_stretch_if_needed, otsu_binarize, PreprocError,
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
