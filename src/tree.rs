use pyo3::{
    create_exception, exceptions::PyException, pyfunction, PyResult,
};

use crate::{instrs::Slot, parse::tcompile, tape::BasicTape as Tape};

type Step = u64;

/**************************************/

create_exception!(tree, TreeSkip, PyException);

#[pyfunction]
pub fn run_for_undefined(
    prog: &str,
    sim_lim: Step,
) -> PyResult<Option<Slot>> {
    let comp = tcompile(prog);

    let mut state = 1;

    let mut tape = Tape::init_stepped();

    for _ in 0..sim_lim {
        let slot = (state, tape.scan);

        let Some(&(color, shift, next_state)) = comp.get(&slot) else {
            return Ok(Some(slot));
        };

        let same = state == next_state;

        if same && tape.at_edge(shift) {
            return Err(TreeSkip::new_err(""));
        }

        let _ = tape.step(shift, color, same);

        if tape.blank() {
            return Err(TreeSkip::new_err(""));
        }

        state = next_state;
    }

    Ok(None)
}
