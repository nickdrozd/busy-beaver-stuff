#![deny(clippy::all)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::redundant_pub_crate)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]

mod graph;
mod parsemod;
mod prover;
mod rules;

use pyo3::prelude::*;

use graph::Graph;
use parsemod::{dcomp_instr, parse, st_str, str_st, tcompile};
use prover::PastConfig;
use rules::{InfiniteRule, RuleLimit, UnknownRule};

#[pymodule]
pub fn rust_stuff(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Graph>()?;

    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(tcompile, m)?)?;
    m.add_function(wrap_pyfunction!(st_str, m)?)?;
    m.add_function(wrap_pyfunction!(str_st, m)?)?;
    m.add_function(wrap_pyfunction!(dcomp_instr, m)?)?;

    m.add_class::<PastConfig>()?;

    m.add_class::<UnknownRule>()?;
    m.add_class::<InfiniteRule>()?;
    m.add_class::<RuleLimit>()?;

    Ok(())
}
