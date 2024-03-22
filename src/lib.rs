#![deny(clippy::all)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![allow(clippy::similar_names)]
#![allow(clippy::redundant_pub_crate)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]

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
    use crate::machine::{run_machine, MachineResult, TermRes};

    #[pymodule_export]
    use crate::parse::{
        erase_slots, halt_slots, init_prog, parse, read_slot, show_instr, show_slot, show_state,
        tcompile, zero_reflexive_slots,
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
