use pyo3::prelude::*;

mod extract_stream;

use extract_stream::{extract_stream_fn, StreamIterator};

/// Python bindings for pdftract-core.
#[pymodule]
fn pdftract(_py: Python, m: &PyModule) -> PyResult<()> {
    // Add the extract_stream function (renamed internally to avoid collision)
    m.add_function(wrap_pyfunction!(extract_stream_fn, m)?)?;
    m.add_class::<StreamIterator>()?;

    Ok(())
}
