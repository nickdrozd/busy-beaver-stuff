#![deny(
    clippy::all,
    clippy::nursery,
    clippy::pedantic,
    clippy::get_unwrap,
    clippy::str_to_string,
    clippy::allow_attributes,
    clippy::std_instead_of_core,
    clippy::if_then_some_else_none,
    clippy::redundant_type_annotations
)]
#![expect(
    clippy::if_not_else,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::module_name_repetitions,
    clippy::redundant_pub_crate
)]

mod blocks;
mod instrs;
mod machine;
mod parse;
mod prover;
mod reason;
mod rules;
mod tape;
mod tree;

use pyo3::pymodule;

#[pymodule]
mod rust_stuff {
    #[pymodule_export]
    use crate::{
        blocks::opt_block_py,
        machine::{
            quick_term_or_rec_py, run_prover, run_quick_machine,
            MachineResult, TermRes,
        },
        parse::{
            parse_to_vec, read_slot, show_comp_py, show_instr,
            show_slot, show_state, tcompile,
        },
        prover::PastConfigs,
        reason::{cant_blank_py, cant_halt_py, cant_spin_out_py},
        tree::tree_progs,
    };
}
