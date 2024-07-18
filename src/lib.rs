#![deny(
    clippy::all,
    clippy::nursery,
    clippy::pedantic,
    clippy::get_unwrap,
    clippy::str_to_string,
    clippy::allow_attributes,
    clippy::unwrap_in_result,
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

/**************************************/

mod wrappers {
    use pyo3::pyfunction;

    use crate::{
        blocks::opt_block,
        instrs::{CompProg, Params},
        machine::quick_term_or_rec,
        parse::{show_comp, tcompile},
        reason::{cant_blank, cant_halt, cant_spin_out, Cycles},
    };

    #[pyfunction]
    pub fn py_cant_halt(prog: &str, cycles: Cycles) -> bool {
        cant_halt(&tcompile(prog), cycles)
    }

    #[pyfunction]
    pub fn py_cant_blank(prog: &str, cycles: Cycles) -> bool {
        cant_blank(&tcompile(prog), cycles)
    }

    #[pyfunction]
    pub fn py_cant_spin_out(prog: &str, cycles: Cycles) -> bool {
        cant_spin_out(&tcompile(prog), cycles)
    }

    #[pyfunction]
    pub fn py_opt_block(prog: &str, steps: usize) -> usize {
        opt_block(&tcompile(prog), steps)
    }

    #[pyfunction]
    pub fn py_quick_term_or_rec(prog: &str, sim_lim: u32) -> bool {
        quick_term_or_rec(&tcompile(prog), sim_lim, false)
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
}

/**************************************/

use pyo3::pymodule;

#[pymodule]
mod rust_stuff {
    #[pymodule_export]
    use crate::{
        machine::{
            run_prover, run_quick_machine, MachineResult, TermRes,
        },
        parse::{
            read_instr, read_slot, show_instr, show_slot, show_state,
            tcompile,
        },
        prover::PastConfigs,
        tree::tree_progs,
        wrappers::{
            py_cant_blank, py_cant_halt, py_cant_spin_out,
            py_opt_block, py_quick_term_or_rec, py_show_comp,
        },
    };
}
