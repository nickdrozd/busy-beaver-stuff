use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
fn rust_stuff(_py: Python, m: &PyModule) -> PyResult<()> {
    Ok(())
}
