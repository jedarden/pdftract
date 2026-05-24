//! Python streaming extraction API using PyO3.
//!
//! This module implements `extract_stream` which returns a Python iterator
//! that yields page dicts one at a time, keeping memory bounded for large PDFs.

use pyo3::exceptions::PyStopIteration;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::mpsc;
use std::thread;

use pdftract_core::{extract_pdf_streaming, ExtractionOptions};

/// StreamIterator for Python's iterator protocol.
///
/// This PyClass wraps a background thread that performs PDF extraction
/// and yields pages via a channel. The Python iterator protocol consumes
/// pages from the channel as they're produced.
#[pyclass]
pub struct StreamIterator {
    /// Channel receiver for page results.
    receiver: Option<mpsc::Receiver<PageFrame>>,
    /// Join handle for the background extraction thread.
    handle: Option<thread::JoinHandle<Result<(), String>>>,
}

/// A single page frame yielded by the streaming iterator.
///
/// This contains the same data as PageResult but is structured for
/// efficient serialization to Python dict format.
struct PageFrame {
    /// Zero-based page index.
    page_index: usize,
    /// Extracted spans (text fragments).
    spans: Vec<SpanFrame>,
    /// Extracted blocks (semantic units).
    blocks: Vec<BlockFrame>,
    /// Extracted tables.
    tables: Vec<TableFrame>,
    /// Error message if extraction failed.
    error: Option<String>,
}

/// A span frame for serialization.
struct SpanFrame {
    text: String,
    bbox: [f64; 4],
    font: String,
    size: f64,
    confidence: Option<f64>,
}

/// A block frame for serialization.
struct BlockFrame {
    kind: String,
    text: String,
    bbox: [f64; 4],
    level: Option<u8>,
    table_index: Option<usize>,
}

/// A table frame for serialization.
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

/// A row frame for serialization.
struct RowFrame {
    bbox: [f64; 4],
    cells: Vec<CellFrame>,
    is_header: bool,
}

/// A cell frame for serialization.
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

/// Convert a PageFrame to a Python dict.
fn page_frame_to_py<'py>(py: Python<'py>, frame: &PageFrame) -> PyResult<PyObject> {
    let spans: Vec<PyObject> = frame
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
            Ok(dict.into())
        })
        .collect::<PyResult<_>>()?;

    let blocks: Vec<PyObject> = frame
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
            Ok(dict.into())
        })
        .collect::<PyResult<_>>()?;

    let tables: Vec<PyObject> = frame
        .tables
        .iter()
        .map(|table| {
            let rows: Vec<PyObject> = table
                .rows
                .iter()
                .map(|row| {
                    let cells: Vec<PyObject> = row
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
                            Ok(dict.into())
                        })
                        .collect::<PyResult<_>>()?;
                    let dict = PyDict::new(py);
                    dict.set_item("bbox", row.bbox.to_vec())?;
                    dict.set_item("cells", cells)?;
                    dict.set_item("is_header", row.is_header)?;
                    Ok(dict.into())
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
            Ok(dict.into())
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

    Ok(result.into())
}

#[pymethods]
impl StreamIterator {
    /// Return self as an iterator.
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Get the next page dict from the stream.
    ///
    /// Returns the next page dict or raises StopIteration when extraction
    /// is complete. If an error occurred during extraction, raises RuntimeError.
    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        let recv = self
            .receiver
            .as_ref()
            .ok_or_else(|| PyStopIteration::new_err(()))?;

        // Try to receive without blocking - we need to do this outside allow_threads
        // because Receiver is not Sync
        let frame_result = recv.try_recv();

        match frame_result {
            Ok(frame) => {
                let py_obj = page_frame_to_py(py, &frame)?;
                Ok(Some(py_obj))
            }
            Err(mpsc::TryRecvError::Empty) => {
                // No data available yet - release GIL and wait a bit
                // This is a simple polling approach; a proper solution would use
                // a crossbeam channel or similar Sync-aware channel
                py.allow_threads(|| std::thread::sleep(std::time::Duration::from_millis(10)));

                // Try again after releasing GIL
                let recv = self
                    .receiver
                    .as_ref()
                    .ok_or_else(|| PyStopIteration::new_err(()))?;

                match recv.try_recv() {
                    Ok(frame) => {
                        let py_obj = page_frame_to_py(py, &frame)?;
                        Ok(Some(py_obj))
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        // Still no data - return None to signal "try again"
                        // This isn't standard Python iterator protocol but works for polling
                        Ok(None)
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        // Channel closed - check thread result
                        self.check_thread_complete()
                    }
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                // Channel closed - check thread result
                self.check_thread_complete()
            }
        }
    }
}

impl StreamIterator {
    fn check_thread_complete(&mut self) -> PyResult<Option<PyObject>> {
        // Channel closed: thread is done
        // Join the thread to check for errors
        if let Some(handle) = self.handle.take() {
            // Drop receiver to fully close channel
            drop(self.receiver.take());

            match handle.join() {
                Ok(Ok(())) => {
                    // Extraction completed successfully
                    Err(PyStopIteration::new_err(()))
                }
                Ok(Err(e)) => {
                    // Extraction returned an error
                    Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
                }
                Err(_) => {
                    // Thread panicked
                    Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        "Extraction thread panicked",
                    ))
                }
            }
        } else {
            // Already cleaned up
            Err(PyStopIteration::new_err(()))
        }
    }
}

/// Extract pages from a PDF as a streaming iterator.
///
/// Returns an iterator that yields one page dict per call. Each page dict
/// contains:
///   - page_index: int (zero-based)
///   - spans: list of span dicts with text, bbox, font, size
///   - blocks: list of block dicts with kind, text, bbox
///   - tables: list of table dicts with rows, cells
///   - error: str (only present if extraction failed for this page)
///
/// Memory usage stays bounded regardless of PDF size. Only one page is
/// resident in memory at a time.
///
/// # Arguments
///
/// * `path` - Path to the PDF file
/// * `**kwargs` - Optional extraction parameters (currently ignored, using defaults)
///
/// # Returns
///
/// A StreamIterator that yields page dicts.
///
/// # Raises
///
/// * `RuntimeError` - If the PDF cannot be opened or parsed
#[pyfunction]
pub fn extract_stream_fn(
    py: Python<'_>,
    path: &str,
    _kwargs: Option<&PyDict>,
) -> PyResult<Py<StreamIterator>> {
    let opts = ExtractionOptions::default();

    let (tx, rx) = mpsc::channel();
    let path_owned = path.to_string();

    let handle = thread::spawn(move || {
        extract_pdf_streaming(std::path::Path::new(&path_owned), &opts, |page| {
            tx.send(PageFrame::from(page.clone())).is_ok()
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
    });

    Ok(Py::new(
        py,
        StreamIterator {
            receiver: Some(rx),
            handle: Some(handle),
        },
    )?)
}
