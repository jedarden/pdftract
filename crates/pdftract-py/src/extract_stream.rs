//! Python streaming extraction API using PyO3.

use pyo3::exceptions::PyStopIteration;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use pdftract_core::{extract_pdf_streaming, ExtractionOptions, ReceiptsMode};
use secrecy::SecretString;

// Type alias for PyO3 owned references
type PyResultAny<'py> = PyResult<Py<PyAny>>;

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

        // Parse password (str) → password: Option<SecretString>
        if let Some(password) = kwargs.get_item("password")? {
            let pwd: String = password.extract()?;
            opts.password = Some(SecretString::new(pwd.into()));
        }

        // Parse max_decompress_gb (int) → max_decompress_bytes: u64
        if let Some(max_gb) = kwargs.get_item("max_decompress_gb")? {
            let gb: u64 = max_gb.extract()?;
            opts.max_decompress_bytes = gb.saturating_mul(1024 * 1024 * 1024);
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

        // Parse pages (str) → pages: Option<String>
        if let Some(pages) = kwargs.get_item("pages")? {
            opts.pages = Some(pages.extract()?);
        }

        // Parse formats (list[str]) - Not implemented yet
        if let Some(formats) = kwargs.get_item("formats")? {
            let _formats: Vec<String> = formats.extract()?;
            // Output format selection is not yet implemented
        }
    }

    Ok(opts)
}

/// StreamIterator for Python's iterator protocol.
#[pyclass]
pub struct StreamIterator {
    receiver: Option<Arc<Mutex<mpsc::Receiver<PageFrame>>>>,
    handle: Option<thread::JoinHandle<Result<(), String>>>,
}

struct PageFrame {
    page_index: usize,
    spans: Vec<SpanFrame>,
    blocks: Vec<BlockFrame>,
    tables: Vec<TableFrame>,
    error: Option<String>,
}

struct SpanFrame {
    text: String,
    bbox: [f64; 4],
    font: String,
    size: f64,
    confidence: Option<f64>,
}

struct BlockFrame {
    kind: String,
    text: String,
    bbox: [f64; 4],
    level: Option<u8>,
    table_index: Option<usize>,
}

struct TableFrame {
    id: String,
    bbox: [f64; 4],
    rows: Vec<RowFrame>,
    header_rows: u32,
    detection_method: String,
    continued: bool,
    continued_from_prev: bool,
    page_index: usize,
}

struct RowFrame {
    bbox: [f64; 4],
    cells: Vec<CellFrame>,
    is_header: bool,
}

struct CellFrame {
    bbox: [f64; 4],
    text: String,
    spans: Vec<usize>,
    row: usize,
    col: usize,
    rowspan: u32,
    colspan: u32,
    is_header_row: bool,
}

impl From<pdftract_core::PageResult> for PageFrame {
    fn from(page: pdftract_core::PageResult) -> Self {
        PageFrame {
            page_index: page.index,
            spans: page.spans.into_iter().map(Into::into).collect(),
            blocks: page.blocks.into_iter().map(Into::into).collect(),
            tables: page.tables.into_iter().map(Into::into).collect(),
            error: page.error,
        }
    }
}

impl From<pdftract_core::SpanJson> for SpanFrame {
    fn from(span: pdftract_core::SpanJson) -> Self {
        SpanFrame {
            text: span.text,
            bbox: span.bbox,
            font: span.font,
            size: span.size,
            confidence: span.confidence.map(|c| c as f64),
        }
    }
}

impl From<pdftract_core::BlockJson> for BlockFrame {
    fn from(block: pdftract_core::BlockJson) -> Self {
        BlockFrame {
            kind: block.kind,
            text: block.text,
            bbox: block.bbox,
            level: block.level,
            table_index: block.table_index,
        }
    }
}

impl From<pdftract_core::TableJson> for TableFrame {
    fn from(table: pdftract_core::TableJson) -> Self {
        TableFrame {
            id: table.id,
            bbox: table.bbox,
            rows: table.rows.into_iter().map(Into::into).collect(),
            header_rows: table.header_rows,
            detection_method: table.detection_method,
            continued: table.continued,
            continued_from_prev: table.continued_from_prev,
            page_index: table.page_index,
        }
    }
}

impl From<pdftract_core::RowJson> for RowFrame {
    fn from(row: pdftract_core::RowJson) -> Self {
        RowFrame {
            bbox: row.bbox,
            cells: row.cells.into_iter().map(Into::into).collect(),
            is_header: row.is_header,
        }
    }
}

impl From<pdftract_core::CellJson> for CellFrame {
    fn from(cell: pdftract_core::CellJson) -> Self {
        CellFrame {
            bbox: cell.bbox,
            text: cell.text,
            spans: cell.spans,
            row: cell.row,
            col: cell.col,
            rowspan: cell.rowspan,
            colspan: cell.colspan,
            is_header_row: cell.is_header_row,
        }
    }
}

fn page_frame_to_py<'py>(py: Python<'py>, frame: &PageFrame) -> PyResultAny<'py> {
    let spans: Vec<Py<PyAny>> = frame
        .spans
        .iter()
        .map(|span| {
            let dict = PyDict::new(py);
            dict.set_item("text", &span.text)?;
            dict.set_item("bbox", span.bbox.to_vec())?;
            dict.set_item("font", &span.font)?;
            dict.set_item("size", span.size)?;
            if let Some(conf) = span.confidence {
                dict.set_item("confidence", conf)?;
            }
            Ok(dict.clone().into())
        })
        .collect::<PyResult<_>>()?;

    let blocks: Vec<Py<PyAny>> = frame
        .blocks
        .iter()
        .map(|block| {
            let dict = PyDict::new(py);
            dict.set_item("kind", &block.kind)?;
            dict.set_item("text", &block.text)?;
            dict.set_item("bbox", block.bbox.to_vec())?;
            if let Some(level) = block.level {
                dict.set_item("level", level)?;
            }
            if let Some(table_idx) = block.table_index {
                dict.set_item("table_index", table_idx)?;
            }
            Ok(dict.clone().into())
        })
        .collect::<PyResult<_>>()?;

    let tables: Vec<Py<PyAny>> = frame
        .tables
        .iter()
        .map(|table| {
            let rows: Vec<Py<PyAny>> = table
                .rows
                .iter()
                .map(|row| {
                    let cells: Vec<Py<PyAny>> = row
                        .cells
                        .iter()
                        .map(|cell| {
                            let dict = PyDict::new(py);
                            dict.set_item("bbox", cell.bbox.to_vec())?;
                            dict.set_item("text", &cell.text)?;
                            dict.set_item("spans", cell.spans.to_vec())?;
                            dict.set_item("row", cell.row)?;
                            dict.set_item("col", cell.col)?;
                            dict.set_item("rowspan", cell.rowspan)?;
                            dict.set_item("colspan", cell.colspan)?;
                            dict.set_item("is_header_row", cell.is_header_row)?;
                            Ok(dict.clone().into())
                        })
                        .collect::<PyResult<_>>()?;
                    let dict = PyDict::new(py);
                    dict.set_item("bbox", row.bbox.to_vec())?;
                    dict.set_item("cells", cells)?;
                    dict.set_item("is_header", row.is_header)?;
                    Ok(dict.clone().into())
                })
                .collect::<PyResult<_>>()?;

            let dict = PyDict::new(py);
            dict.set_item("id", &table.id)?;
            dict.set_item("bbox", table.bbox.to_vec())?;
            dict.set_item("rows", rows)?;
            dict.set_item("header_rows", table.header_rows)?;
            dict.set_item("detection_method", &table.detection_method)?;
            dict.set_item("continued", table.continued)?;
            dict.set_item("continued_from_prev", table.continued_from_prev)?;
            dict.set_item("page_index", table.page_index)?;
            Ok(dict.clone().into())
        })
        .collect::<PyResult<_>>()?;

    let result = PyDict::new(py);
    result.set_item("page_index", frame.page_index)?;
    result.set_item("spans", spans)?;
    result.set_item("blocks", blocks)?;
    result.set_item("tables", tables)?;
    if let Some(ref err) = frame.error {
        result.set_item("error", err)?;
    }

    Ok(result.clone().into())
}

#[pymethods]
impl StreamIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        // Check if receiver is still available
        let recv_opt = self.receiver.take();
        if recv_opt.is_none() {
            return Err(PyStopIteration::new_err(()));
        }
        let recv = recv_opt.unwrap();

        // Try non-blocking recv first - if data is available, return immediately
        {
            let recv_guard = recv.lock().unwrap();
            match recv_guard.try_recv() {
                Ok(frame) => {
                    // Drop guard before moving recv
                    drop(recv_guard);
                    // Restore receiver for next iteration
                    self.receiver = Some(recv);
                    // GIL must be held for pythonize
                    let py_obj = page_frame_to_py(py, &frame)?;
                    return Ok(Some(py_obj));
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Sender is done - check thread result
                    return self.check_thread_complete();
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Fall through to blocking recv below
                }
            }
        }

        // Channel is empty - do blocking recv with GIL released
        let recv_clone = Arc::clone(&recv);
        let frame = py.allow_threads(move || {
            let recv_guard = recv_clone.lock().unwrap();
            recv_guard.recv()
        });

        // Restore receiver for next iteration (unless this is the end)
        self.receiver = Some(recv);

        match frame {
            Ok(frame) => {
                let py_obj = page_frame_to_py(py, &frame)?;
                Ok(Some(py_obj))
            }
            Err(mpsc::RecvError) => self.check_thread_complete(),
        }
    }
}

impl StreamIterator {
    fn check_thread_complete(&mut self) -> PyResult<Option<Py<PyAny>>> {
        if let Some(handle) = self.handle.take() {
            self.receiver.take();

            match handle.join() {
                Ok(Ok(())) => Err(PyStopIteration::new_err(())),
                Ok(Err(e)) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e)),
                Err(_) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Extraction thread panicked",
                )),
            }
        } else {
            Err(PyStopIteration::new_err(()))
        }
    }
}

/// Extract pages from a PDF as a streaming iterator.
///
/// This function returns a Python iterator that yields one page dict per page.
/// Each dict contains the page's spans, blocks, and tables.
///
/// # Arguments
///
/// * `path` - Path to the PDF file (local file or HTTPS URL)
/// * `**kwargs` - Optional extraction options (see ALLOWED_KWARGS)
///
/// # Returns
///
/// A StreamIterator that yields page dicts.
///
/// # Examples
///
/// ```python
/// import pdftract
///
/// # Stream extraction
/// for page in pdftract.extract_stream("document.pdf"):
///     print(f"Page {page['page_index']}: {len(page['spans'])} spans")
/// ```
#[pyfunction]
pub fn extract_stream_fn(
    py: Python<'_>,
    path: &str,
    kwargs: Option<&PyDict>,
) -> PyResult<Py<StreamIterator>> {
    // Parse kwargs into ExtractionOptions with strict validation
    let opts = parse_kwargs(kwargs)?;

    let (tx, rx) = mpsc::channel();
    let pdf_path = std::path::PathBuf::from(path);
    let opts_owned = opts.clone();

    let handle = thread::spawn(move || {
        extract_pdf_streaming(&pdf_path, &opts_owned, |page| {
            tx.send(PageFrame::from(page.clone())).is_ok()
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
    });

    Ok(Py::new(
        py,
        StreamIterator {
            receiver: Some(Arc::new(Mutex::new(rx))),
            handle: Some(handle),
        },
    )?)
}
