#![deny(
    clippy::all,
    clippy::nursery,
    clippy::pedantic,
    clippy::get_unwrap,
    clippy::ref_patterns,
    clippy::str_to_string,
    clippy::clone_on_ref_ptr,
    clippy::same_name_method,
    clippy::allow_attributes,
    clippy::unwrap_in_result,
    clippy::partial_pub_fields,
    clippy::unused_trait_names,
    clippy::std_instead_of_core,
    clippy::if_then_some_else_none,
    clippy::unneeded_field_pattern,
    clippy::redundant_type_annotations
)]
#![expect(clippy::cast_possible_truncation)]
#![allow(clippy::enum_glob_use, clippy::should_panic_without_expect)]

mod blocks;
mod cps;
mod ctl;
mod graph;
mod instrs;
mod machine;
mod macros;
mod prover;
mod reason;
mod rules;
mod segment;
mod tape;
mod tree;
mod wrappers;

#[cfg(test)]
mod test;

/**************************************/

use pyo3::pymodule;

#[pymodule]
mod rust_stuff {
    #[pymodule_export]
    use crate::{
        instrs::{
            read_instr, read_slot, show_instr, show_slot, show_state,
        },
        machine::{
            run_prover, run_quick_machine, MachineResult, TermRes,
        },
        prover::PastConfigs,
        wrappers::{
            py_cant_blank, py_cant_halt, py_cant_spin_out,
            py_cps_cant_blank, py_cps_cant_halt, py_cps_cant_spin_out,
            py_ctl_cant_blank, py_ctl_cant_halt, py_ctl_cant_spin_out,
            py_is_connected, py_opt_block, py_quick_term_or_rec,
            py_segment_cant_blank, py_segment_cant_halt,
            py_segment_cant_spin_out, py_show_comp, tcompile,
            tree_progs, BackwardResult,
        },
    };
}
