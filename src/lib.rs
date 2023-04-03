mod parse;
mod prover;

use pyo3::prelude::*;

use parse::{dcomp_instr, parse as parse_fn, st_str, str_st, tcompile};
use prover::PastConfig;

#[pymodule]
fn rust_stuff(_py: Python, m: &PyModule) -> PyResult<()> {
    // parse
    m.add_function(wrap_pyfunction!(parse_fn, m)?)?;
    m.add_function(wrap_pyfunction!(tcompile, m)?)?;
    m.add_function(wrap_pyfunction!(st_str, m)?)?;
    m.add_function(wrap_pyfunction!(str_st, m)?)?;
    m.add_function(wrap_pyfunction!(dcomp_instr, m)?)?;

    // prover
    m.add_class::<PastConfig>()?;

    Ok(())
}
