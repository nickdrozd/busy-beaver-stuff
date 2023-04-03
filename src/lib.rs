mod prover;

use pyo3::prelude::*;

use prover::PastConfig;

#[pymodule]
fn rust_stuff(_py: Python, m: &PyModule) -> PyResult<()> {
    // prover
    m.add_class::<PastConfig>()?;

    Ok(())
}
