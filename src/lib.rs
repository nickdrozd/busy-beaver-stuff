#![deny(
    clippy::all,
    clippy::nursery,
    clippy::pedantic,
    clippy::get_unwrap,
    clippy::str_to_string,
    clippy::allow_attributes,
    clippy::unwrap_in_result,
    clippy::partial_pub_fields,
    clippy::std_instead_of_core,
    clippy::if_then_some_else_none,
    clippy::unneeded_field_pattern,
    clippy::redundant_type_annotations
)]
#![expect(
    clippy::cast_possible_truncation,
    clippy::module_name_repetitions,
    clippy::redundant_pub_crate
)]
#![allow(clippy::should_panic_without_expect)]

mod blocks;
mod graph;
mod instrs;
mod machine;
mod macros;
mod prover;
mod reason;
mod rules;
mod tape;
mod tree;

/**************************************/

mod wrappers {
    use pyo3::pyfunction;

    use crate::{
        blocks::opt_block,
        graph::is_connected,
        instrs::{CompProg, Params, Parse, State},
        machine::quick_term_or_rec,
        reason::{cant_blank, cant_halt, cant_spin_out, Depth, Step},
    };

    #[pyfunction]
    pub fn py_cant_halt(prog: &str, depth: Depth) -> Option<Step> {
        cant_halt(&CompProg::from_str(prog), depth)
    }

    #[pyfunction]
    pub fn py_cant_blank(prog: &str, depth: Depth) -> Option<Step> {
        cant_blank(&CompProg::from_str(prog), depth)
    }

    #[pyfunction]
    pub fn py_cant_spin_out(prog: &str, depth: Depth) -> Option<Step> {
        cant_spin_out(&CompProg::from_str(prog), depth)
    }

    #[pyfunction]
    pub fn py_is_connected(prog: &str, states: State) -> bool {
        is_connected(&CompProg::from_str(prog), states)
    }

    #[pyfunction]
    pub fn py_opt_block(prog: &str, steps: usize) -> usize {
        opt_block(&CompProg::from_str(prog), steps)
    }

    #[pyfunction]
    pub fn py_quick_term_or_rec(prog: &str, sim_lim: usize) -> bool {
        quick_term_or_rec(&CompProg::from_str(prog), sim_lim, false)
    }

    #[pyfunction]
    #[pyo3(signature = (comp, params=None))]
    #[expect(clippy::needless_pass_by_value)]
    pub fn py_show_comp(
        comp: CompProg,
        params: Option<Params>,
    ) -> String {
        show_comp(&comp, params)
    }

    #[pyfunction]
    pub fn tcompile(prog: &str) -> CompProg {
        CompProg::from_str(prog)
    }

    pub fn show_comp(
        comp: &CompProg,
        params: Option<Params>,
    ) -> String {
        comp.show(params)
    }
}

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
        tree::tree_progs,
        wrappers::{
            py_cant_blank, py_cant_halt, py_cant_spin_out,
            py_is_connected, py_opt_block, py_quick_term_or_rec,
            py_show_comp, tcompile,
        },
    };
}
