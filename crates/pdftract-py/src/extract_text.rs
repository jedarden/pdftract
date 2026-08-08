//! Python extract_text() entry point using PyO3.
//!
//! This module provides the extract_text() function that returns plain text
//! from a PDF, with kwargs parsing into ExtractionOptions, GIL release during
//! extraction, and direct String return (no intermediate dict).

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::path::Path;

use pdftract_core::options::ReceiptsMode;
use pdftract_core::{extract_text, ExtractionOptions};

/// Allowed kwarg names for strict validation.
const ALLOWED_KWARGS: &[&str] = &[
    "ocr",
    "ocr_language",
    "include_invisible",
    "extract_forms",
    "extract_attachments",
    "readability_threshold",
    "password",
    "max_decompress_gb",
    "full_render",
    "receipts",
    "cache_dir",
    "pages",
    "formats",
];

/// Parse Python kwargs into ExtractionOptions.
///
/// This function performs strict validation: unknown kwargs raise PdftractError
/// to catch typos early rather than silently ignoring them.
fn parse_kwargs(kwargs: Option<&PyDict>) -> PyResult<ExtractionOptions> {
    let mut opts = ExtractionOptions::default();

    if let Some(kwargs) = kwargs {
        // Validate that all kwargs are in the allowlist
        for key in kwargs.keys() {
            let key_str: String = key.extract()?;
            if !ALLOWED_KWARGS.contains(&key_str.as_str()) {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                    "Unknown keyword argument '{}'. Allowed: {}",
                    key_str,
                    ALLOWED_KWARGS.join(", ")
                )));
            }
        }

        // Parse ocr (bool) - No-op for now, OCR is controlled by feature flag
        if let Some(ocr) = kwargs.get_item("ocr")? {
            let _ocr: bool = ocr.extract()?;
            // OCR is controlled by the 'ocr' feature flag in pdftract-core
            // This kwarg is accepted for API compatibility but has no effect
        }

        // Parse ocr_language (list[str] or comma-string)
        if let Some(lang) = kwargs.get_item("ocr_language")? {
            if let Ok(lang_list) = lang.extract::<Vec<String>>() {
                opts.ocr_language = lang_list;
            } else if let Ok(lang_str) = lang.extract::<String>() {
                // Split on comma if provided as string
                opts.ocr_language = lang_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "ocr_language must be a list of strings or a comma-separated string",
                ));
            }
        }

        // Parse include_invisible (bool) → output.include_invisible
        if let Some(include_invisible) = kwargs.get_item("include_invisible")? {
            opts.output.include_invisible = include_invisible.extract()?;
        }

        // Parse password (str) → password: Option<SecretString>
        if let Some(password) = kwargs.get_item("password")? {
            let pwd: String = password.extract()?;
            opts.password = Some(secrecy::SecretString::new(pwd.into()));
        }

        // Parse max_decompress_gb (int) → max_decompress_bytes: u64
        if let Some(max_gb) = kwargs.get_item("max_decompress_gb")? {
            let gb: u64 = max_gb.extract()?;
            opts.max_decompress_bytes = gb.saturating_mul(1024 * 1024 * 1024);
        }

        // Parse pages (str) → pages: Option<String>
        if let Some(pages) = kwargs.get_item("pages")? {
            opts.pages = Some(pages.extract()?);
        }

        // Parse extract_forms (bool) - No-op, forms are always extracted
        if let Some(extract_forms) = kwargs.get_item("extract_forms")? {
            let _extract_forms: bool = extract_forms.extract()?;
            // Forms are always extracted; this kwarg is accepted for API compatibility
        }

        // Parse extract_attachments (bool) - No-op, attachments are always extracted
        if let Some(extract_attachments) = kwargs.get_item("extract_attachments")? {
            let _extract_attachments: bool = extract_attachments.extract()?;
            // Attachments are always extracted; this kwarg is accepted for API compatibility
        }

        // Parse readability_threshold (float) - Not implemented yet
        if let Some(readability_threshold) = kwargs.get_item("readability_threshold")? {
            let _readability_threshold: f64 = readability_threshold.extract()?;
            // Readability threshold is not yet implemented in pdftract-core
        }

        // Parse full_render (bool) → full_render: bool
        if let Some(full_render) = kwargs.get_item("full_render")? {
            opts.full_render = full_render.extract()?;
        }

        // Parse receipts (str) → receipts: ReceiptsMode
        if let Some(receipts) = kwargs.get_item("receipts")? {
            let receipts_str: String = receipts.extract()?;
            opts.receipts = ReceiptsMode::from_str(&receipts_str)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
        }

        // Parse cache_dir (str) - Not implemented yet
        if let Some(cache_dir) = kwargs.get_item("cache_dir")? {
            let _cache_dir: String = cache_dir.extract()?;
            // Cache dir is not yet implemented in pdftract-core
        }

        // Parse formats (list[str]) - Not implemented yet
        if let Some(formats) = kwargs.get_item("formats")? {
            let _formats: Vec<String> = formats.extract()?;
            // Output format selection is not yet implemented
        }
    }

    Ok(opts)
}

/// Extract plain text from a PDF, returning a String.
///
/// This is the fast path for RAG ingest pipelines that just want the text body.
/// It returns a bare String, avoiding the cost of serializing the full Document
/// to JSON and re-parsing in Python.
///
/// This function is wrapped by `#[pyfunction]` in lib.rs; do not add the attribute here.
///
/// # Arguments
///
/// * `py` - Python GIL token
/// * `path` - Path to the PDF file (local file or HTTPS URL)
/// * `kwargs` - Optional extraction options (see ALLOWED_KWARGS)
///
/// # Returns
///
/// A Python string containing the extracted text. Span texts are concatenated
/// in reading order, each followed by a newline (matching `pdftract extract --text`).
///
/// # Examples
///
/// ```python
/// import pdftract
///
/// # Basic text extraction
/// text = pdftract.extract_text("document.pdf")
/// print(f"Extracted {len(text)} characters")
///
/// # With page range
/// text = pdftract.extract_text("doc.pdf", pages="1-5")
///
/// # With invisible text included
/// text = pdftract.extract_text("doc.pdf", include_invisible=True)
///
/// # With password for encrypted PDF
/// text = pdftract.extract_text("encrypted.pdf", password="secret123")
/// ```
///
/// # Errors
///
/// - `PdftractError` - Base class for all PDF processing errors
/// - `EncryptionError` - PDF is encrypted and password is wrong or missing
/// - `CorruptPdfError` - PDF file is malformed or invalid
/// - `SourceUnreachableError` - Remote PDF could not be fetched
/// - `TlsError` - TLS handshake failed for remote PDF
///
/// # Thread Safety
///
/// The GIL is released during the blocking extraction operation, allowing
/// other Python threads to run concurrently.
pub fn extract_text_fn(py: Python<'_>, path: &str, kwargs: Option<&PyDict>) -> PyResult<String> {
    // Parse kwargs into ExtractionOptions with strict validation
    let opts = parse_kwargs(kwargs)?;

    // Resolve path (local file or URL)
    let pdf_path = Path::new(path);

    // Run extraction with GIL released so other Python threads can run
    let text = py
        .allow_threads(|| extract_text(pdf_path, &opts))
        .map_err(|e| {
            // Map anyhow::Error to appropriate Python exception
            let msg = e.to_string();
            let err_str = msg.to_lowercase();

            if err_str.contains("encrypted") || err_str.contains("password") {
                PyErr::new::<crate::EncryptionError, _>(msg)
            } else if err_str.contains("corrupt") || err_str.contains("invalid") {
                PyErr::new::<crate::CorruptPdfError, _>(msg)
            } else if err_str.contains("tls")
                || err_str.contains("certificate")
                || err_str.contains("ssl")
            {
                PyErr::new::<crate::TlsError, _>(msg)
            } else if err_str.contains("network") || err_str.contains("interrupted") {
                PyErr::new::<crate::RemoteFetchInterruptedError, _>(msg)
            } else if err_str.contains("unreachable") || err_str.contains("not found") {
                PyErr::new::<crate::SourceUnreachableError, _>(msg)
            } else {
                PyErr::new::<crate::PdftractError, _>(msg)
            }
        })?;

    Ok(text)
}

// Note: Python-level tests for kwargs parsing are in the Python test suite.
// Rust unit tests that require Python::with_gil() are not included here to avoid
// linking issues with the Python interpreter.
