use pyo3::prelude::*;

/// Python bindings for pdftract-core.
#[pymodule]
fn pdftract(_m: &Bound<'_, PyModule>) -> PyResult<()> {
    Ok(())
}
