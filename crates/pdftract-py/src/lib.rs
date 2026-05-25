//! Python bindings for pdftract-core.
//!
//! This module provides idiomatic Python bindings via PyO3, exposing
//! the 9 contract methods and the 8-class exception hierarchy.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::path::Path;

// Import base64 for decoding attachment data in PyO3 bindings
use base64::engine::general_purpose::STANDARD;
use base64::engine::Engine;

// Type alias for PyO3 owned references
type PyResultAny<'py> = PyResult<Py<PyAny>>;

mod extract_stream;

use extract_stream::{extract_stream_fn, StreamIterator};

// Re-export core types and functions
use pdftract_core::{
    extract_pdf, extract_pdf_streaming, AttachmentJson, BeadJson, ExtractionOptions, PageResult,
    TableJson, ThreadJson,
};

// ============================================================================
// Exception hierarchy
// ============================================================================

/// Base exception for all pdftract errors.
#[pyclass(name = "PdftractError")]
#[derive(Debug)]
pub struct PyPdftractError {
    #[pyo3(get, set)]
    message: String,
}

impl From<anyhow::Error> for PyPdftractError {
    fn from(err: anyhow::Error) -> Self {
        PyPdftractError {
            message: err.to_string(),
        }
    }
}

#[pymethods]
impl PyPdftractError {
    fn __str__(&self) -> String {
        self.message.clone()
    }

    fn __repr__(&self) -> String {
        format!("PdftractError({})", self.message)
    }
}

// Corrupt PDF error
#[pyclass(name = "CorruptPdfError")]
#[derive(Debug)]
pub struct PyCorruptPdfError {
    #[pyo3(get, set)]
    message: String,
}

#[pymethods]
impl PyCorruptPdfError {
    fn __str__(&self) -> String {
        self.message.clone()
    }
}

// Encryption error
#[pyclass(name = "EncryptionError")]
#[derive(Debug)]
pub struct PyEncryptionError {
    #[pyo3(get, set)]
    message: String,
}

#[pymethods]
impl PyEncryptionError {
    fn __str__(&self) -> String {
        self.message.clone()
    }
}

// Source unreachable error
#[pyclass(name = "SourceUnreachableError")]
#[derive(Debug)]
pub struct PySourceUnreachableError {
    #[pyo3(get, set)]
    message: String,
}

#[pymethods]
impl PySourceUnreachableError {
    fn __str__(&self) -> String {
        self.message.clone()
    }
}

// Remote fetch interrupted error
#[pyclass(name = "RemoteFetchInterruptedError")]
#[derive(Debug)]
pub struct PyRemoteFetchInterruptedError {
    #[pyo3(get, set)]
    message: String,
}

#[pymethods]
impl PyRemoteFetchInterruptedError {
    fn __str__(&self) -> String {
        self.message.clone()
    }
}

// TLS error
#[pyclass(name = "TlsError")]
#[derive(Debug)]
pub struct PyTlsError {
    #[pyo3(get, set)]
    message: String,
}

#[pymethods]
impl PyTlsError {
    fn __str__(&self) -> String {
        self.message.clone()
    }
}

// Receipt verify error
#[pyclass(name = "ReceiptVerifyError")]
#[derive(Debug)]
pub struct PyReceiptVerifyError {
    #[pyo3(get, set)]
    message: String,
}

#[pymethods]
impl PyReceiptVerifyError {
    fn __str__(&self) -> String {
        self.message.clone()
    }
}

// Unsupported operation error
#[pyclass(name = "UnsupportedOperationError")]
#[derive(Debug)]
pub struct PyUnsupportedOperationError {
    #[pyo3(get, set)]
    message: String,
}

#[pymethods]
impl PyUnsupportedOperationError {
    fn __str__(&self) -> String {
        self.message.clone()
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Convert a Rust error to the appropriate Python exception.
fn map_error_to_py(py: Python, err: anyhow::Error) -> PyErr {
    let msg = err.to_string();
    let err_str = msg.to_lowercase();

    // Map to specific exception based on error message
    if err_str.contains("encrypted") || err_str.contains("password") {
        PyErr::new::<PyEncryptionError, _>(msg)
    } else if err_str.contains("corrupt") || err_str.contains("invalid") {
        PyErr::new::<PyCorruptPdfError, _>(msg)
    } else if err_str.contains("tls") || err_str.contains("certificate") || err_str.contains("ssl")
    {
        PyErr::new::<PyTlsError, _>(msg)
    } else if err_str.contains("network") || err_str.contains("interrupted") {
        PyErr::new::<PyRemoteFetchInterruptedError, _>(msg)
    } else if err_str.contains("unreachable") || err_str.contains("not found") {
        PyErr::new::<PySourceUnreachableError, _>(msg)
    } else {
        PyErr::new::<PyPdftractError, _>(msg)
    }
}

/// Convert Python kwargs to ExtractionOptions.
fn kwargs_to_options(kwargs: Option<&PyDict>) -> PyResult<ExtractionOptions> {
    let opts = ExtractionOptions::default();
    // For now, just return default options
    // TODO: Parse kwargs to set options when ExtractionOptions has those fields
    Ok(opts)
}

// ============================================================================
// Contract method: extract
// ============================================================================

/// Extract text and structure from a PDF.
///
/// Returns a Document object containing pages with spans, blocks, and tables.
#[pyfunction]
#[pyo3(name = "extract")]
fn extract_py<'py>(py: Python<'py>, path: &str, kwargs: Option<&PyDict>) -> PyResultAny<'py> {
    let opts = kwargs_to_options(kwargs)?;
    let pdf_path = Path::new(path);

    // Run extraction
    let result = extract_pdf(pdf_path, &opts).map_err(|e| map_error_to_py(py, e))?;

    // Convert ExtractionResult to Python dict
    let dict = PyDict::new(py);

    // Add metadata
    let metadata = PyDict::new(py);
    metadata.set_item("page_count", result.metadata.page_count)?;
    metadata.set_item("span_count", result.metadata.span_count)?;
    metadata.set_item("block_count", result.metadata.block_count)?;
    if let Some(cache_status) = result.metadata.cache_status {
        metadata.set_item("cache_status", cache_status)?;
    }
    dict.set_item("metadata", metadata)?;

    // Add pages
    let pages: PyResult<Vec<Py<PyAny>>> = result
        .pages
        .into_iter()
        .map(|page| page_to_py(py, page))
        .collect();
    dict.set_item("pages", pages?)?;

    // Add attachments (with base64 data decoded to bytes)
    let attachments: PyResult<Vec<Py<PyAny>>> = result
        .attachments
        .into_iter()
        .map(|attachment| attachment_to_py(py, attachment))
        .collect();
    dict.set_item("attachments", attachments?)?;

    // Add threads (as Python list of dicts)
    let threads: PyResult<Vec<Py<PyAny>>> = result
        .threads
        .into_iter()
        .map(|thread| thread_to_py(py, thread))
        .collect();
    dict.set_item("threads", threads?)?;

    Ok(dict.clone().into())
}

/// Convert a Bead to a Python dict with two keys (page_index, rect).
///
/// Per the bead spec, beads are simple 2-key dicts for compactness.
fn bead_to_py<'py>(py: Python<'py>, bead: BeadJson) -> PyResultAny<'py> {
    let dict = PyDict::new(py);
    dict.set_item("page_index", bead.page_index)?;
    dict.set_item("rect", bead.rect)?;
    Ok(dict.clone().into())
}

/// Convert a Thread to a Python dict with title, author, subject, keywords, and beads.
///
/// This converts the full ThreadJson structure to a Python dict, including
/// the list of beads (each bead is a 2-key dict via bead_to_py).
fn thread_to_py<'py>(py: Python<'py>, thread: ThreadJson) -> PyResultAny<'py> {
    let dict = PyDict::new(py);

    dict.set_item("title", thread.title)?;
    dict.set_item("author", thread.author)?;
    dict.set_item("subject", thread.subject)?;
    dict.set_item("keywords", thread.keywords)?;

    // Convert beads to Python list of 2-key dicts
    let beads: PyResult<Vec<Py<PyAny>>> = thread
        .beads
        .into_iter()
        .map(|bead| bead_to_py(py, bead))
        .collect();
    dict.set_item("beads", beads?)?;

    Ok(dict.clone().into())
}

// ============================================================================
// Contract method: extract_text
// ============================================================================

#[pyfunction]
fn extract_text(py: Python, path: &str, kwargs: Option<&PyDict>) -> PyResult<String> {
    let result = extract_py(py, path, kwargs)?;
    let dict = result.downcast::<PyDict>(py)?;
    let pages = dict
        .get_item("pages")?
        .unwrap()
        .downcast::<pyo3::types::PyList>()?;

    let mut text = String::new();
    for page in pages.iter() {
        let page_dict = page.downcast::<PyDict>()?;
        let spans = page_dict
            .get_item("spans")?
            .unwrap()
            .downcast::<pyo3::types::PyList>()?;

        for span in spans.iter() {
            let span_dict = span.downcast::<PyDict>()?;
            if let Some(text_obj) = span_dict.get_item("text")? {
                let span_text: String = text_obj.extract()?;
                text.push_str(&span_text);
                text.push(' ');
            }
        }
    }

    Ok(text)
}

// ============================================================================
// Contract method: extract_markdown (stub)
// ============================================================================

#[pyfunction]
fn extract_markdown(py: Python, path: &str, kwargs: Option<&PyDict>) -> PyResult<String> {
    // For now, just return extract_text output
    // TODO: Implement proper markdown conversion
    extract_text(py, path, kwargs)
}

// ============================================================================
// Contract method: search (stub)
// ============================================================================

#[pyfunction]
fn search<'py>(
    py: Python<'py>,
    _path: &str,
    pattern: &str,
    _kwargs: Option<&PyDict>,
) -> PyResultAny<'py> {
    // For now, extract and return empty match list
    // TODO: Implement proper regex search
    let dict = PyDict::new(py);
    dict.set_item("pattern", pattern)?;

    // Return an empty match list for now
    let matches = pyo3::types::PyList::empty(py);
    dict.set_item("matches", matches)?;

    Ok(dict.clone().into())
}

// ============================================================================
// Contract method: get_metadata
// ============================================================================

#[pyfunction]
fn get_metadata<'py>(py: Python<'py>, path: &str, kwargs: Option<&PyDict>) -> PyResultAny<'py> {
    let result = extract_py(py, path, kwargs)?;
    let dict = result.downcast::<PyDict>(py)?;
    let metadata = dict.get_item("metadata")?.unwrap();
    Ok(metadata.clone().into())
}

// ============================================================================
// Contract method: hash (stub)
// ============================================================================

#[pyfunction]
fn hash(_py: Python, _path: &str, _kwargs: Option<&PyDict>) -> PyResult<String> {
    // Stub implementation - should compute fingerprint
    // For now, return a placeholder
    Ok(format!("pdftract-v1:{}", "0".repeat(64)))
}

// ============================================================================
// Contract method: classify (stub)
// ============================================================================

#[pyfunction]
fn classify<'py>(py: Python<'py>, _path: &str) -> PyResultAny<'py> {
    // Stub implementation - should classify page type
    let dict = PyDict::new(py);
    dict.set_item("class_name", "Unknown")?;
    dict.set_item("confidence", 0.0f64)?;
    Ok(dict.clone().into())
}

// ============================================================================
// Contract method: verify_receipt (stub)
// ============================================================================

#[pyfunction]
fn verify_receipt(_py: Python, _path: &str, _receipt_dict: &PyDict) -> PyResult<bool> {
    // Stub implementation - should verify receipt
    // For now, return false
    Ok(false)
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
fn pdftract(_py: Python, m: &PyModule) -> PyResult<()> {
    // Add exception classes
    m.add_class::<PyPdftractError>()?;
    m.add_class::<PyCorruptPdfError>()?;
    m.add_class::<PyEncryptionError>()?;
    m.add_class::<PySourceUnreachableError>()?;
    m.add_class::<PyRemoteFetchInterruptedError>()?;
    m.add_class::<PyTlsError>()?;
    m.add_class::<PyReceiptVerifyError>()?;
    m.add_class::<PyUnsupportedOperationError>()?;

    // Add extract_stream function
    m.add_function(wrap_pyfunction!(extract_stream_fn, m)?)?;
    m.add_class::<StreamIterator>()?;

    // Add main extraction function
    m.add_function(wrap_pyfunction!(extract_py, m)?)?;
    m.add_function(wrap_pyfunction!(extract_text, m)?)?;
    m.add_function(wrap_pyfunction!(extract_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add_function(wrap_pyfunction!(get_metadata, m)?)?;
    m.add_function(wrap_pyfunction!(hash, m)?)?;
    m.add_function(wrap_pyfunction!(classify, m)?)?;
    m.add_function(wrap_pyfunction!(verify_receipt, m)?)?;

    Ok(())
}
