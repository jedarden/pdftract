//! Python bindings for pdftract-core.
//!
//! This module provides idiomatic Python bindings via PyO3, exposing
//! the 9 contract methods and the 8-class exception hierarchy.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::path::Path;
use std::sync::OnceLock;

// Type alias for PyO3 owned references
type PyResultAny<'py> = PyResult<Py<PyAny>>;

mod extract;
mod extract_stream;
mod extract_text;
mod extract_markdown;

use extract::extract as extract_fn;
use extract_stream::{extract_stream_fn, StreamIterator};
use extract_text::extract_text_fn;
use extract_markdown::extract_markdown_fn;

// Re-export core types
use pdftract_core::{AttachmentJson, ExtractionOptions, PageResult, TableJson};
use pdftract_core::sdk::{search as sdk_search, SearchMatch};

// ============================================================================
// PyPdfProcessor - Python-facing PDF processor
// ============================================================================

/// Main PDF processor for Python bindings.
///
/// This struct provides a high-level interface for processing PDFs
/// through the Python bindings. More functionality will be added in
/// subsequent tasks.
pub struct PyPdfProcessor {
    /// Internal path to the PDF file
    _path: std::path::PathBuf,
}

impl PyPdfProcessor {
    /// Create a new PyPdfProcessor for the given PDF path.
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { _path: path }
    }
}

// Import diagnostics for error code mapping
use pdftract_core::diagnostics::DIAGNOSTIC_CATALOG;

// ============================================================================
// Exception hierarchy
// ============================================================================

/// The exception types this module raises.
///
/// The hierarchy is *defined* once, in `python/pdftract/exceptions.py`; this
/// module resolves those very class objects at import time and raises them. The
/// alternative — `pyo3::create_exception!` here — would mint a second, unrelated
/// `PdftractError`, and `except pdftract.PdftractError` would never catch a
/// native failure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ErrorKind {
    Base,
    CorruptPdf,
    Encryption,
    SourceUnreachable,
    RemoteFetchInterrupted,
    Tls,
    ReceiptVerify,
    UnsupportedOperation,
}

static EXCEPTION_TYPES: OnceLock<ExceptionTypes> = OnceLock::new();

#[derive(Clone)]
struct ExceptionTypes {
    base: Py<PyAny>,
    corrupt_pdf: Py<PyAny>,
    encryption: Py<PyAny>,
    source_unreachable: Py<PyAny>,
    remote_fetch_interrupted: Py<PyAny>,
    tls: Py<PyAny>,
    receipt_verify: Py<PyAny>,
    unsupported_operation: Py<PyAny>,
}

impl ExceptionTypes {
    /// Import the hierarchy from `pdftract.exceptions`, the package's public
    /// source of truth for exception identity.
    ///
    /// Called from `#[pymodule]`; importing `pdftract.exceptions` while
    /// `pdftract/__init__.py` is still executing is safe because that module
    /// never reads anything from the parent package.
    fn from_python(py: Python<'_>) -> PyResult<Self> {
        let exceptions = py.import("pdftract.exceptions")?;
        Ok(Self {
            base: exceptions.getattr("PdftractError")?.into(),
            corrupt_pdf: exceptions.getattr("CorruptPdfError")?.into(),
            encryption: exceptions.getattr("EncryptionError")?.into(),
            source_unreachable: exceptions.getattr("SourceUnreachableError")?.into(),
            remote_fetch_interrupted: exceptions
                .getattr("RemoteFetchInterruptedError")?
                .into(),
            tls: exceptions.getattr("TlsError")?.into(),
            receipt_verify: exceptions.getattr("ReceiptVerifyError")?.into(),
            unsupported_operation: exceptions.getattr("UnsupportedOperationError")?.into(),
        })
    }

    /// The resolved types, or the init failure that prevented resolving them.
    fn get() -> PyResult<&'static Self> {
        EXCEPTION_TYPES.get().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "pdftract exception types not initialised; pdftract._native must be \
                 imported as part of the pdftract package",
            )
        })
    }

    fn get_type(&self, kind: ErrorKind) -> &Py<PyAny> {
        match kind {
            ErrorKind::Base => &self.base,
            ErrorKind::CorruptPdf => &self.corrupt_pdf,
            ErrorKind::Encryption => &self.encryption,
            ErrorKind::SourceUnreachable => &self.source_unreachable,
            ErrorKind::RemoteFetchInterrupted => &self.remote_fetch_interrupted,
            ErrorKind::Tls => &self.tls,
            ErrorKind::ReceiptVerify => &self.receipt_verify,
            ErrorKind::UnsupportedOperation => &self.unsupported_operation,
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Get the hint for a diagnostic code from the catalog.
fn get_hint_for_code(code: &str) -> Option<&'static str> {
    DIAGNOSTIC_CATALOG
        .iter()
        .find(|info| info.code.to_string() == code)
        .map(|info| info.suggested_action)
}

/// Classify an extraction failure into the exception kind and diagnostic code
/// it should surface as.
///
/// `anyhow::Error` carries no structured diagnostic, so this reads the message.
fn classify_error(msg: &str) -> (ErrorKind, Option<String>) {
    let msg = msg.to_lowercase();

    if msg.contains("encrypted") || msg.contains("password") {
        let diag_code = if msg.contains("wrong") || msg.contains("incorrect") {
            "ENCRYPTION_WRONG_PASSWORD"
        } else {
            "ENCRYPTION_UNSUPPORTED"
        };
        (ErrorKind::Encryption, Some(diag_code.to_string()))
    } else if msg.contains("corrupt") || msg.contains("invalid") {
        (ErrorKind::CorruptPdf, Some("STRUCT_INVALID_NAME".to_string()))
    } else if msg.contains("tls") || msg.contains("certificate") || msg.contains("ssl") {
        (ErrorKind::Tls, Some("REMOTE_TLS_ERROR".to_string()))
    } else if msg.contains("network") || msg.contains("interrupted") {
        (
            ErrorKind::RemoteFetchInterrupted,
            Some("REMOTE_FETCH_INTERRUPTED".to_string()),
        )
    } else if msg.contains("unreachable") || msg.contains("not found") {
        (
            ErrorKind::SourceUnreachable,
            Some("REMOTE_HOST_UNREACHABLE".to_string()),
        )
    } else {
        (ErrorKind::Base, None)
    }
}

/// Convert a Rust error to the appropriate Python exception with attributes.
///
/// Maps `anyhow::Error` to one of the exception types defined in
/// `pdftract.exceptions` and sets the code, page_index, and hint attributes
/// derived from the error message.
fn map_error_to_py(py: Python, err: anyhow::Error) -> PyErr {
    let msg = err.to_string();
    let (kind, code) = classify_error(&msg);
    let hint = code.as_deref().and_then(get_hint_for_code);

    let types = match ExceptionTypes::get() {
        Ok(types) => types,
        Err(uninitialised) => return uninitialised,
    };
    let exc_type = match types.get_type(kind).downcast::<pyo3::types::PyType>(py) {
        Ok(exc_type) => exc_type,
        Err(e) => return PyErr::from(e),
    };
    let exc = PyErr::from_type(exc_type, (msg,));

    // Attach the diagnostic attributes to the instance PyErr::from_type built
    let instance = exc.value(py);
    if let Some(ref c) = code {
        let _ = instance.setattr("code", c);
    }
    let _ = instance.setattr("page_index", None::<u32>);
    if let Some(ref h) = hint {
        let _ = instance.setattr("hint", h);
    }

    exc
}

/// Convert Python kwargs to ExtractionOptions.
fn kwargs_to_options(kwargs: Option<&PyDict>) -> PyResult<ExtractionOptions> {
    let mut opts = ExtractionOptions::default();

    if let Some(kwargs) = kwargs {
        // Parse pages parameter
        if let Some(pages) = kwargs.get_item("pages")? {
            let pages_str: Option<String> = pages.extract()?;
            if let Some(range) = pages_str {
                opts.pages = Some(range);
            }
        }

        // Parse receipts parameter
        if let Some(receipts) = kwargs.get_item("receipts")? {
            let receipts_str: Option<String> = receipts.extract()?;
            if let Some(mode) = receipts_str {
                opts.receipts = pdftract_core::options::ReceiptsMode::from_str(&mode)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
            }
        }
    }

    Ok(opts)
}

// ============================================================================
// Contract method: extract_text
// ============================================================================

/// Extract plain text from a PDF, returning a String.
///
/// This is the fast path for RAG ingest pipelines that just want the text body.
/// It returns a bare String, avoiding the cost of serializing the full Document
/// to JSON and re-parsing in Python.
///
/// See the extract_text module for full documentation.
#[pyfunction(name = "extract_text")]
#[pyo3(signature = (path, **kwargs))]
fn py_extract_text(py: Python, path: &str, kwargs: Option<&PyDict>) -> PyResult<String> {
    extract_text_fn(py, path, kwargs)
}

// ============================================================================
// Contract method: extract_markdown
// ============================================================================

/// Extract Markdown from a PDF, returning a String.
///
/// This function returns the document formatted as Markdown, preserving
/// document structure with headers, links, and other formatting.
///
/// See the extract_markdown module for full documentation.
#[pyfunction(name = "extract_markdown")]
#[pyo3(signature = (path, **kwargs))]
fn py_extract_markdown(py: Python, path: &str, kwargs: Option<&PyDict>) -> PyResult<String> {
    extract_markdown_fn(py, path, kwargs)
}

// ============================================================================
// Contract method: search
// ============================================================================

/// Convert one SDK search result to the Python-facing match dictionary.
///
/// Keep this mapping next to the binding so the Python contract remains a
/// direct representation of [`pdftract_core::sdk::SearchMatch`]. In
/// particular, `bbox` is exposed as a four-element Python list, matching the
/// other geometry fields produced by this module.
fn search_match_to_py<'py>(py: Python<'py>, search_match: SearchMatch) -> PyResultAny<'py> {
    let match_dict = PyDict::new(py);
    match_dict.set_item("page_index", search_match.page_index)?;
    match_dict.set_item("span_index", search_match.span_index)?;
    match_dict.set_item("text", search_match.text)?;
    match_dict.set_item("bbox", search_match.bbox.to_vec())?;
    Ok(match_dict.into())
}

/// Search for text patterns in a PDF.
///
/// This function searches for a pattern in the PDF text content and returns
/// all matches with their locations (page, span, bounding box).
///
/// # Arguments
///
/// * `path` - Path to the PDF file
/// * `pattern` - Text pattern to search for
/// * `kwargs` - Optional search parameters:
///   - `case_insensitive` (bool) - Ignore case when matching (default: false)
///   - `regex` (bool) - Treat pattern as a regular expression (default: false)
///   - `whole_word` (bool) - Match only whole words (default: false)
///
/// # Returns
///
/// A dictionary with:
/// * `pattern` - The search pattern
/// * `matches` - List of match dictionaries, each with:
///   - `page_index` (int) - Page number where match was found
///   - `span_index` (int) - Span index within the page
///   - `text` (str) - Matched text content
///   - `bbox` (list[float]) - Bounding box [x0, y0, x1, y1]
#[pyfunction]
#[pyo3(signature = (path, pattern, **kwargs))]
fn search<'py>(
    py: Python<'py>,
    path: &str,
    pattern: &str,
    kwargs: Option<&PyDict>,
) -> PyResultAny<'py> {
    // Parse search options from kwargs
    let case_insensitive = kwargs
        .and_then(|k| k.get_item("case_insensitive").ok().flatten())
        .and_then(|v| v.extract::<bool>().ok())
        .unwrap_or(false);

    let use_regex = kwargs
        .and_then(|k| k.get_item("regex").ok().flatten())
        .and_then(|v| v.extract::<bool>().ok())
        .unwrap_or(false);

    let whole_word = kwargs
        .and_then(|k| k.get_item("whole_word").ok().flatten())
        .and_then(|v| v.extract::<bool>().ok())
        .unwrap_or(false);

    // Call the SDK search function
    let pdf_path = Path::new(path);
    let result = sdk_search(pdf_path, pattern, case_insensitive, use_regex, whole_word);

    // Map errors to Python exceptions
    let matches = match result {
        Ok(matches) => matches,
        Err(err) => {
            return Err(map_error_to_py(py, err));
        }
    };

    // Build the result dictionary
    let dict = PyDict::new(py);
    dict.set_item("pattern", pattern)?;

    // Convert matches to Python list of dicts
    let matches_list = pyo3::types::PyList::empty(py);
    for search_match in matches {
        let match_dict = search_match_to_py(py, search_match)?;
        matches_list.append(match_dict)?;
    }
    dict.set_item("matches", matches_list)?;

    Ok(dict.clone().into())
}

// ============================================================================
// Contract method: get_metadata
// ============================================================================

#[pyfunction]
fn get_metadata<'py>(py: Python<'py>, path: &str, kwargs: Option<&PyDict>) -> PyResultAny<'py> {
    let result = extract_fn(py, path, kwargs)?;
    let dict = result.downcast::<PyDict>(py)?;
    let metadata = dict.get_item("metadata")?.unwrap();
    Ok(metadata.clone().into())
}

// ============================================================================
// Contract method: hash (stub)
// ============================================================================

#[pyfunction]
fn hash(py: Python, path: &str, _kwargs: Option<&PyDict>) -> PyResult<String> {
    pdftract_core::sdk::hash(std::path::Path::new(path)).map_err(|e| map_error_to_py(py, e))
}

// ============================================================================
// Contract method: classify
// ============================================================================

#[pyfunction]
fn classify<'py>(py: Python<'py>, path: &str, page_index: Option<usize>) -> PyResultAny<'py> {
    // Default to first page if page_index not provided
    let page_idx = page_index.unwrap_or(0);

    // Call the SDK classify function
    let classification = pdftract_core::sdk::classify(std::path::Path::new(path), page_idx)
        .map_err(|e| map_error_to_py(py, e))?;

    // Convert to Python dict
    let dict = PyDict::new(py);
    dict.set_item("class_name", classification.class.as_type_str())?;
    dict.set_item("confidence", f64::from(classification.confidence))?;
    Ok(dict.into())
}

// ============================================================================
// Contract method: verify_receipt (stub)
// ============================================================================

#[pyfunction]
fn verify_receipt(py: Python, path: &str, receipt_dict: &PyDict) -> PyResult<bool> {
    // The core verifier owns the receipt protocol and its mismatch handling.
    // Serialize the Python mapping to a private file so the binding uses that
    // implementation instead of returning the old unconditional stub value.
    let receipt: serde_json::Value = pythonize::depythonize(receipt_dict).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
            "receipt must be a JSON-compatible mapping: {e}"
        ))
    })?;
    let mut receipt_file = tempfile::NamedTempFile::new().map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyOSError, _>(format!(
            "failed to create temporary receipt file: {e}"
        ))
    })?;
    serde_json::to_writer(receipt_file.as_file_mut(), &receipt).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "failed to serialize receipt: {e}"
        ))
    })?;

    let verification = pdftract_core::sdk::verify_receipt_from_path(
        Path::new(path),
        receipt_file.path(),
    )
    .map_err(|e| map_error_to_py(py, e))?;

    Ok(verification.is_ok())
}

// ============================================================================
// Helper: Convert PageResult to Python dict
// ============================================================================

fn page_to_py<'py>(py: Python<'py>, page: PageResult) -> PyResultAny<'py> {
    let dict = PyDict::new(py);

    dict.set_item("page_index", page.index)?;

    // Convert spans
    let spans: PyResult<Vec<Py<PyAny>>> = page
        .spans
        .into_iter()
        .map(|span| {
            let span_dict = PyDict::new(py);
            span_dict.set_item("text", span.text)?;
            span_dict.set_item("bbox", span.bbox.to_vec())?;
            span_dict.set_item("font", span.font)?;
            span_dict.set_item("size", span.size)?;
            if let Some(conf) = span.confidence {
                span_dict.set_item("confidence", conf as f64)?;
            }
            Ok(span_dict.clone().into())
        })
        .collect();
    dict.set_item("spans", spans?)?;

    // Convert blocks
    let blocks: PyResult<Vec<Py<PyAny>>> = page
        .blocks
        .into_iter()
        .map(|block| {
            let block_dict = PyDict::new(py);
            block_dict.set_item("kind", block.kind)?;
            block_dict.set_item("text", block.text)?;
            block_dict.set_item("bbox", block.bbox.to_vec())?;
            if let Some(level) = block.level {
                block_dict.set_item("level", level)?;
            }
            if let Some(table_index) = block.table_index {
                block_dict.set_item("table_index", table_index)?;
            }
            Ok(block_dict.clone().into())
        })
        .collect();
    dict.set_item("blocks", blocks?)?;

    // Convert tables
    let tables: PyResult<Vec<Py<PyAny>>> = page
        .tables
        .into_iter()
        .map(|table| table_to_py(py, table))
        .collect();
    dict.set_item("tables", tables?)?;

    if let Some(error) = page.error {
        dict.set_item("error", error)?;
    }

    Ok(dict.clone().into())
}

fn table_to_py<'py>(py: Python<'py>, table: TableJson) -> PyResultAny<'py> {
    let dict = PyDict::new(py);

    dict.set_item("id", table.id)?;
    dict.set_item("bbox", table.bbox.to_vec())?;

    // Convert rows
    let rows: PyResult<Vec<Py<PyAny>>> = table
        .rows
        .into_iter()
        .map(|row| {
            let row_dict = PyDict::new(py);
            row_dict.set_item("bbox", row.bbox.to_vec())?;
            row_dict.set_item("is_header", row.is_header)?;

            // Convert cells
            let cells: PyResult<Vec<Py<PyAny>>> = row
                .cells
                .into_iter()
                .map(|cell| {
                    let cell_dict = PyDict::new(py);
                    cell_dict.set_item("bbox", cell.bbox.to_vec())?;
                    cell_dict.set_item("text", cell.text)?;
                    cell_dict.set_item("spans", cell.spans.to_vec())?;
                    cell_dict.set_item("row", cell.row)?;
                    cell_dict.set_item("col", cell.col)?;
                    cell_dict.set_item("rowspan", cell.rowspan)?;
                    cell_dict.set_item("colspan", cell.colspan)?;
                    cell_dict.set_item("is_header_row", cell.is_header_row)?;
                    Ok(cell_dict.clone().into())
                })
                .collect();
            row_dict.set_item("cells", cells?)?;

            Ok(row_dict.clone().into())
        })
        .collect();
    dict.set_item("rows", rows?)?;

    dict.set_item("header_rows", table.header_rows)?;
    dict.set_item("detection_method", table.detection_method)?;
    dict.set_item("continued", table.continued)?;
    dict.set_item("continued_from_prev", table.continued_from_prev)?;
    dict.set_item("page_index", table.page_index)?;

    Ok(dict.clone().into())
}

fn attachment_to_py<'py>(py: Python<'py>, attachment: AttachmentJson) -> PyResultAny<'py> {
    let dict = PyDict::new(py);

    dict.set_item("name", attachment.name)?;
    dict.set_item("description", attachment.description)?;
    dict.set_item("mime_type", attachment.mime_type)?;
    dict.set_item("size", attachment.size)?;
    dict.set_item("created", attachment.created)?;
    dict.set_item("modified", attachment.modified)?;
    dict.set_item("checksum_md5", attachment.checksum_md5)?;
    dict.set_item("truncated", attachment.truncated)?;

    // Convert base64 data to bytes (PyO3 will decode the base64 string)
    if let Some(base64_data) = attachment.data {
        use base64::engine::general_purpose::STANDARD;
        use base64::engine::Engine;

        match STANDARD.decode(&base64_data) {
            Ok(bytes) => {
                let py_bytes = pyo3::types::PyBytes::new(py, &bytes);
                dict.set_item("data", py_bytes)?;
            }
            Err(_) => {
                // If base64 decoding fails, set data to None
                dict.set_item("data", py.None())?;
            }
        }
    } else {
        dict.set_item("data", py.None())?;
    }

    Ok(dict.clone().into())
}

// ============================================================================
// PyO3 module definition
// ============================================================================

#[pymodule]
fn _native(py: Python, m: &PyModule) -> PyResult<()> {
    // Resolve the exception hierarchy defined in pdftract.exceptions and
    // re-expose it here, so `pdftract._native.PdftractError` is the very same
    // object `pdftract.PdftractError` is.
    let types = ExceptionTypes::from_python(py)?;
    if EXCEPTION_TYPES.set(types).is_err() {
        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            "pdftract exception types already initialised",
        ));
    }
    let types = EXCEPTION_TYPES.get().expect("just set above");
    m.add("PdftractError", types.base.clone_ref(py))?;
    m.add("EncryptionError", types.encryption.clone_ref(py))?;
    m.add("CorruptPdfError", types.corrupt_pdf.clone_ref(py))?;
    m.add(
        "SourceUnreachableError",
        types.source_unreachable.clone_ref(py),
    )?;
    m.add(
        "RemoteFetchInterruptedError",
        types.remote_fetch_interrupted.clone_ref(py),
    )?;
    m.add("TlsError", types.tls.clone_ref(py))?;
    m.add("ReceiptVerifyError", types.receipt_verify.clone_ref(py))?;
    m.add(
        "UnsupportedOperationError",
        types.unsupported_operation.clone_ref(py),
    )?;

    // Add extract_stream function
    m.add_function(wrap_pyfunction!(extract_stream_fn, m)?)?;
    m.add_class::<StreamIterator>()?;

    // Add main extraction functions
    m.add_function(wrap_pyfunction!(extract::extract, m)?)?;
    m.add_function(wrap_pyfunction!(py_extract_text, m)?)?;
    m.add_function(wrap_pyfunction!(py_extract_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add_function(wrap_pyfunction!(get_metadata, m)?)?;
    m.add_function(wrap_pyfunction!(hash, m)?)?;
    m.add_function(wrap_pyfunction!(classify, m)?)?;
    m.add_function(wrap_pyfunction!(verify_receipt, m)?)?;

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================
// Note: Python-level tests (exception hierarchy, kwargs parsing, etc.) are in the
// Python test suite (tests/ directory). Rust unit tests that require Python::with_gil()
// are not included here to avoid linking issues with the Python interpreter.
// ============================================================================
