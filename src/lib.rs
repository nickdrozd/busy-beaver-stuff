mod graph;
mod parsemod;
mod prover;

use pyo3::prelude::*;

use graph::reduce_graph;
use parsemod::*;
use prover::*;

#[pymodule]
fn rust_stuff(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(reduce_graph, m)?)?;

    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(tcompile, m)?)?;
    m.add_function(wrap_pyfunction!(st_str, m)?)?;
    m.add_function(wrap_pyfunction!(str_st, m)?)?;
    m.add_function(wrap_pyfunction!(dcomp_instr, m)?)?;

    m.add_class::<PastConfig>()?;

    Ok(())
}
