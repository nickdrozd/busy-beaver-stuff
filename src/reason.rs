use std::collections::HashMap;

use pyo3::prelude::*;

use crate::instrs::{Color, CompProg, Instr, Shift, State};
use crate::parse::{parse as prim_parse, tcompile};
use crate::tape::{Tape, TupleTape};

/**************************************/

#[pyfunction]
pub fn reason_parse(prog: &str) -> HashMap<State, Vec<Instr>> {
    let mut program = HashMap::new();

    for (state, instrs) in prim_parse(prog).iter().enumerate() {
        program.insert(
            state as State,
            instrs.iter().filter_map(|instr| *instr).collect(),
        );
    }

    program
}

/**************************************/

type Step = u64;

#[pyclass]
pub struct BackstepMachineHalt {
    comp: CompProg,
}

#[pyclass]
pub struct BackstepMachineBlank {
    comp: CompProg,
    blanks: HashMap<State, Step>,
}

#[pyclass]
pub struct BackstepMachineSpinout {
    comp: CompProg,
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

        let mut tape = Tape::from_tuples(init_tape);

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

        let mut tape = Tape::from_tuples(init_tape);

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

        let mut tape = Tape::from_tuples(init_tape);

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

            state = next_state;
        }

        None
    }
}
