#![deny(
    clippy::all,
    clippy::nursery,
    clippy::pedantic,
    clippy::get_unwrap,
    clippy::str_to_string,
    clippy::std_instead_of_core,
    clippy::std_instead_of_alloc,
    clippy::if_then_some_else_none
)]
#![allow(
    dead_code,
    non_local_definitions,
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
    use crate::blocks::{measure_blocks, unroll_tape};

    #[pymodule_export]
    use crate::machine::{
        quick_term_or_rec, run_quick_machine, MachineResult, TermRes,
    };

    #[pymodule_export]
    use crate::parse::{
        init_prog, parse_to_vec, read_slot, show_instr, show_slot,
        show_state, tcompile,
    };

    #[pymodule_export]
    use crate::prover::PastConfigs;

    #[pymodule_export]
    use crate::reason::{cant_blank, cant_halt, cant_spin_out};

    #[pymodule_export]
    use crate::rules::{InfiniteRule, RuleLimit, UnknownRule};

    #[pymodule_export]
    use crate::tree::{run_for_undefined, TreeSkip};
}
