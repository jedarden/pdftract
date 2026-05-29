//! Python bindings for pdftract-core.
//!
//! This module provides idiomatic Python bindings via PyO3, exposing
//! the 9 contract methods and the 8-class exception hierarchy.

use pyo3::prelude::*;
use pyo3::types::PyDict;

// Type alias for PyO3 owned references
type PyResultAny<'py> = PyResult<Py<PyAny>>;

mod extract;
mod extract_stream;
mod extract_text;

use extract::extract as extract_fn;
use extract_stream::{extract_stream_fn, StreamIterator};
use extract_text::extract_text_fn;

// Re-export core types
use pdftract_core::{AttachmentJson, ExtractionOptions, PageResult, TableJson};

// Import diagnostics for error code mapping
use pdftract_core::diagnostics::DIAGNOSTIC_CATALOG;

// ============================================================================
// Exception hierarchy
// ============================================================================

// Use PyO3's create_exception! macro to create proper exception types
// with Python inheritance. EncryptionError inherits from PdftractError,
// and all others inherit from PdftractError.
pyo3::create_exception!(pdftract, PdftractError, pyo3::exceptions::PyException);
pyo3::create_exception!(pdftract, EncryptionError, PdftractError);
pyo3::create_exception!(pdftract, CorruptPdfError, PdftractError);
pyo3::create_exception!(pdftract, SourceUnreachableError, PdftractError);
pyo3::create_exception!(pdftract, RemoteFetchInterruptedError, PdftractError);
pyo3::create_exception!(pdftract, TlsError, PdftractError);
pyo3::create_exception!(pdftract, ReceiptVerifyError, PdftractError);
pyo3::create_exception!(pdftract, UnsupportedOperationError, PdftractError);

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

/// Convert a Rust error to the appropriate Python exception with attributes.
///
/// This function maps anyhow::Error to the appropriate Python exception type
/// and sets the code, page_index, and hint attributes from the error context.
/// Since anyhow::Error doesn't directly expose Diagnostic information, we
/// parse the error message to identify the diagnostic code and set attributes
/// accordingly.
fn map_error_to_py(py: Python, err: anyhow::Error) -> PyErr {
    let msg = err.to_string();
    let err_str = msg.to_lowercase();

    // Determine exception type, diagnostic code, and hint based on error message
    let (code, hint): (Option<String>, Option<String>) = if err_str.contains("encrypted")
        || err_str.contains("password")
    {
        let diag_code = if err_str.contains("wrong") || err_str.contains("incorrect") {
            "ENCRYPTION_WRONG_PASSWORD"
        } else {
            "ENCRYPTION_UNSUPPORTED"
        };
        (
            Some(diag_code.to_string()),
            get_hint_for_code(diag_code).map(|h| h.to_string()),
        )
    } else if err_str.contains("corrupt") || err_str.contains("invalid") {
        (
            Some("STRUCT_INVALID_NAME".to_string()),
            get_hint_for_code("STRUCT_INVALID_NAME").map(|h| h.to_string()),
        )
    } else if err_str.contains("tls") || err_str.contains("certificate") || err_str.contains("ssl")
    {
        (
            Some("REMOTE_TLS_ERROR".to_string()),
            get_hint_for_code("REMOTE_TLS_ERROR").map(|h| h.to_string()),
        )
    } else if err_str.contains("network") || err_str.contains("interrupted") {
        (
            Some("REMOTE_FETCH_INTERRUPTED".to_string()),
            get_hint_for_code("REMOTE_FETCH_INTERRUPTED").map(|h| h.to_string()),
        )
    } else if err_str.contains("unreachable") || err_str.contains("not found") {
        (
            Some("REMOTE_HOST_UNREACHABLE".to_string()),
            get_hint_for_code("REMOTE_HOST_UNREACHABLE").map(|h| h.to_string()),
        )
    } else {
        (None, None)
    };

    // Map to specific exception based on error message
    // Create PyErr and set attributes on the instance
    let PyErr = if err_str.contains("encrypted") || err_str.contains("password") {
        EncryptionError::new_err(msg)
    } else if err_str.contains("corrupt") || err_str.contains("invalid") {
        CorruptPdfError::new_err(msg)
    } else if err_str.contains("tls") || err_str.contains("certificate") || err_str.contains("ssl")
    {
        TlsError::new_err(msg)
    } else if err_str.contains("network") || err_str.contains("interrupted") {
        RemoteFetchInterruptedError::new_err(msg)
    } else if err_str.contains("unreachable") || err_str.contains("not found") {
        SourceUnreachableError::new_err(msg)
    } else {
        PdftractError::new_err(msg)
    };

    // Set attributes on the exception instance
    // We need to get the instance and set attributes using Python's setattr
    let instance = PyErr.value(py);
    if let Some(ref c) = code {
        let _ = instance.setattr("code", c);
    }
    let _ = instance.setattr("page_index", None::<u32>);
    if let Some(ref h) = hint {
        let _ = instance.setattr("hint", h);
    }

    PyErr
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
// Contract method: extract_markdown (stub)
// ============================================================================

#[pyfunction]
fn extract_markdown(py: Python, path: &str, kwargs: Option<&PyDict>) -> PyResult<String> {
    // For now, just return extract_text output
    // TODO: Implement proper markdown conversion
    extract_text_fn(py, path, kwargs)
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
    let result = extract_fn(py, path, kwargs)?;
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
fn pdftract(py: Python, m: &PyModule) -> PyResult<()> {
    // Add exception classes with proper Python inheritance
    m.add("PdftractError", py.get_type::<PdftractError>())?;
    m.add("EncryptionError", py.get_type::<EncryptionError>())?;
    m.add("CorruptPdfError", py.get_type::<CorruptPdfError>())?;
    m.add(
        "SourceUnreachableError",
        py.get_type::<SourceUnreachableError>(),
    )?;
    m.add(
        "RemoteFetchInterruptedError",
        py.get_type::<RemoteFetchInterruptedError>(),
    )?;
    m.add("TlsError", py.get_type::<TlsError>())?;
    m.add("ReceiptVerifyError", py.get_type::<ReceiptVerifyError>())?;
    m.add(
        "UnsupportedOperationError",
        py.get_type::<UnsupportedOperationError>(),
    )?;

    // Add extract_stream function
    m.add_function(wrap_pyfunction!(extract_stream_fn, m)?)?;
    m.add_class::<StreamIterator>()?;

    // Add main extraction functions
    m.add_function(wrap_pyfunction!(extract::extract, m)?)?;
    m.add_function(wrap_pyfunction!(py_extract_text, m)?)?;
    m.add_function(wrap_pyfunction!(extract_markdown, m)?)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exception_hierarchy() {
        // Test that EncryptionError inherits from PdftractError
        Python::with_gil(|py| {
            let pdftract_err = PdftractError::new_err("test error");
            let encryption_err = EncryptionError::new_err("encrypted");

            // Both should be instances of PdftractError
            let pdftract_err_type = py.get_type::<PdftractError>();
            assert!(pdftract_err
                .value(py)
                .is_instance(&pdftract_err_type)
                .unwrap());
            assert!(encryption_err
                .value(py)
                .is_instance(&pdftract_err_type)
                .unwrap());
        });
    }

    #[test]
    fn test_exception_attributes() {
        // Test that exception attributes are set correctly
        Python::with_gil(|py| {
            let err = EncryptionError::new_err("PDF is encrypted");
            let instance = err.value(py);

            // Set attributes
            instance.setattr("code", "ENCRYPTION_UNSUPPORTED").unwrap();
            instance.setattr("page_index", None::<u32>).unwrap();
            instance
                .setattr("hint", "Supply the password keyword argument")
                .unwrap();

            // Verify attributes
            let code: Option<String> = instance.getattr("code").unwrap().extract().unwrap();
            let page_index: Option<u32> =
                instance.getattr("page_index").unwrap().extract().unwrap();
            let hint: Option<String> = instance.getattr("hint").unwrap().extract().unwrap();

            assert_eq!(code, Some("ENCRYPTION_UNSUPPORTED".to_string()));
            assert_eq!(page_index, None);
            assert_eq!(
                hint,
                Some("Supply the password keyword argument".to_string())
            );
        });
    }
}
