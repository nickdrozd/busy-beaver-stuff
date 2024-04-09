#![deny(clippy::all)]
#![warn(clippy::nursery, clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::module_name_repetitions,
    clippy::redundant_pub_crate,
    clippy::similar_names
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

use pyo3::prelude::*;

#[pymodule]
mod rust_stuff {
    #[pymodule_export]
    use crate::blocks::{measure_blocks, unroll_tape};

    #[pymodule_export]
    use crate::machine::{quick_term_or_rec, run_machine, MachineResult, TermRes};

    #[pymodule_export]
    use crate::parse::{
        comp_thic, comp_thin, init_prog, parse_to_vec, read_slot, show_instr, show_slot, show_state,
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
