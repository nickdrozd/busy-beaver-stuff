#![deny(clippy::all)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::redundant_pub_crate)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]

mod blocks;
mod instrs;
mod parse;
mod prover;
mod rules;

use pyo3::prelude::*;

use blocks::{measure_blocks, unroll_tape};
use parse::{parse as parse_fn, read_slot, show_instr, show_slot, show_state, tcompile};
use prover::PastConfigs;
use rules::{InfiniteRule, RuleLimit, UnknownRule};

#[pymodule]
fn rust_stuff(py: Python, m: &PyModule) -> PyResult<()> {
    // blocks
    m.add_function(wrap_pyfunction!(measure_blocks, m)?)?;
    m.add_function(wrap_pyfunction!(unroll_tape, m)?)?;

    // parse
    m.add_function(wrap_pyfunction!(parse_fn, m)?)?;
    m.add_function(wrap_pyfunction!(tcompile, m)?)?;
    m.add_function(wrap_pyfunction!(show_state, m)?)?;
    m.add_function(wrap_pyfunction!(show_slot, m)?)?;
    m.add_function(wrap_pyfunction!(read_slot, m)?)?;
    m.add_function(wrap_pyfunction!(show_instr, m)?)?;

    // prover
    m.add_class::<PastConfigs>()?;

    // rules
    m.add("UnknownRule", py.get_type::<UnknownRule>())?;
    m.add("InfiniteRule", py.get_type::<InfiniteRule>())?;
    m.add("RuleLimit", py.get_type::<RuleLimit>())?;

    Ok(())
}
