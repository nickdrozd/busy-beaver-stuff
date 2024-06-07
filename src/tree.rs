use pyo3::{
    create_exception, exceptions::PyException, pyfunction, PyResult,
};

use crate::{
    instrs::{Color, Instr, Slot, State},
    parse::tcompile,
    tape::BasicTape as Tape,
};

type Step = u64;

/**************************************/

#[pyfunction]
pub fn make_instrs(states: State, colors: Color) -> Vec<Instr> {
    let mut instrs = vec![];

    let shifts = [false, true];

    for color in 0..colors {
        for shift in shifts {
            for state in 0..states {
                instrs.push((color, shift, state));
            }
        }
    }

    instrs
}

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
