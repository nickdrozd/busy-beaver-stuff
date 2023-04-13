#![deny(clippy::all)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::redundant_pub_crate)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]

mod instrs;
mod parse;
mod prover;
mod rules;

use pyo3::prelude::*;

use parse::{dcomp_instr, parse as parse_fn, st_str, str_st, tcompile};
use prover::PastConfig;
use rules::{calculate_diff, make_rule, InfiniteRule, RuleLimit, UnknownRule};

#[pymodule]
fn rust_stuff(py: Python, m: &PyModule) -> PyResult<()> {
    // parse
    m.add_function(wrap_pyfunction!(parse_fn, m)?)?;
    m.add_function(wrap_pyfunction!(tcompile, m)?)?;
    m.add_function(wrap_pyfunction!(st_str, m)?)?;
    m.add_function(wrap_pyfunction!(str_st, m)?)?;
    m.add_function(wrap_pyfunction!(dcomp_instr, m)?)?;

    // prover
    m.add_class::<PastConfig>()?;

    // rules
    m.add("UnknownRule", py.get_type::<UnknownRule>())?;
    m.add("InfiniteRule", py.get_type::<InfiniteRule>())?;
    m.add("RuleLimit", py.get_type::<RuleLimit>())?;
    m.add_function(wrap_pyfunction!(calculate_diff, m)?)?;
    m.add_function(wrap_pyfunction!(make_rule, m)?)?;

    Ok(())
}
