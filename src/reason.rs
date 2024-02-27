use std::collections::HashMap;

use pyo3::prelude::*;

use crate::instrs::{Color, Prog, Shift, State};
use crate::parse::tcompile;
use crate::tape::{BackstepTape, TupleTape};

type Step = u64;

/**************************************/

#[pyclass]
pub struct BackstepMachineHalt {
    comp: Prog,
}

#[pyclass]
pub struct BackstepMachineBlank {
    comp: Prog,
    blanks: HashMap<State, Step>,
}

#[pyclass]
pub struct BackstepMachineSpinout {
    comp: Prog,
}

#[pymethods]
impl BackstepMachineHalt {
    #[new]
    fn new(prog: &str) -> Self {
        Self {
            comp: tcompile(prog),
        }
    }

    fn backstep_run(
        &mut self,
        sim_lim: Step,
        init_tape: TupleTape,
        mut state: State,
        shift: Shift,
        color: Color,
    ) -> Option<Step> {
        let mut step = 0;

        let mut tape = BackstepTape::from_tuples(init_tape);

        tape.backstep(shift, color);

        for _ in 0..sim_lim {
            let Some(&(color, shift, next_state)) = self.comp.get(&(state, tape.scan)) else {
                return Some(step);
            };

            let same = state == next_state;

            if same && tape.at_edge(shift) {
                break;
            }

            let stepped = tape.step(shift, color, same);

            step += stepped;

            if next_state == -1 {
                return Some(step);
            }

            state = next_state;
        }

        None
    }
}

#[pymethods]
impl BackstepMachineBlank {
    #[new]
    fn new(prog: &str) -> Self {
        Self {
            comp: tcompile(prog),
            blanks: HashMap::new(),
        }
    }

    fn backstep_run(
        &mut self,
        sim_lim: Step,
        init_tape: TupleTape,
        mut state: State,
        shift: Shift,
        color: Color,
    ) -> Option<Step> {
        let mut step = 0;

        let mut tape = BackstepTape::from_tuples(init_tape);

        tape.backstep(shift, color);

        for _ in 0..sim_lim {
            let Some(&(color, shift, next_state)) = self.comp.get(&(state, tape.scan)) else {
                break;
            };

            let same = state == next_state;

            if same && tape.at_edge(shift) {
                break;
            }

            let stepped = tape.step(shift, color, same);

            step += stepped;

            if next_state == -1 {
                break;
            }

            state = next_state;

            if color == 0 && tape.blank() {
                if self.blanks.contains_key(&state) {
                    break;
                }

                self.blanks.insert(state, step);

                if state == 0 {
                    break;
                }
            }
        }

        self.blanks.drain().map(|(_, value)| value).min()
    }
}

#[pymethods]
impl BackstepMachineSpinout {
    #[new]
    fn new(prog: &str) -> Self {
        Self {
            comp: tcompile(prog),
        }
    }

    fn backstep_run(
        &mut self,
        sim_lim: Step,
        init_tape: TupleTape,
        mut state: State,
        shift: Shift,
        color: Color,
    ) -> Option<Step> {
        let mut step = 0;

        let mut tape = BackstepTape::from_tuples(init_tape);

        tape.backstep(shift, color);

        for _ in 0..sim_lim {
            let Some(&(color, shift, next_state)) = self.comp.get(&(state, tape.scan)) else {
                break;
            };

            let same = state == next_state;

            if same && tape.at_edge(shift) {
                return Some(step);
            }

            let stepped = tape.step(shift, color, same);

            step += stepped;

            if next_state == -1 {
                break;
            }

            state = next_state;
        }

        None
    }
}
