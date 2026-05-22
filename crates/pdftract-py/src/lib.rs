use pyo3::prelude::*;

/// Python bindings for pdftract-core.
#[pymodule]
fn pdftract(_py: Python, _m: &PyModule) -> PyResult<()> {
    Ok(())
}
